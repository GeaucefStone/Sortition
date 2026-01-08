use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt;
use std::fs;
use std::hash::{Hash, Hasher};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use chrono::{Local, NaiveDate, Datelike};
use dirs;
use lazy_static::lazy_static;
use rand::Rng;
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use std::sync::RwLock;
use thiserror::Error as ThisError;

// ──────────────────────────────────────────────────────────────
// Configuration Models
// ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RosterConfig {
    /// Registry types (e.g., ["citizens", "workers", "volunteers"])
    pub registry_types: Vec<String>,
    
    /// Maximum selections per person before they're considered "maxed out"
    pub max_selections: u32,
    
    /// Maximum people per markdown file
    pub max_people_per_file: usize,
    
    /// Length of roster IDs (in characters)
    pub roster_length: usize,
    
    /// Date format for FOLDER names ONLY (chrono format)
    /// Default: "%Y-%m-%d" (YYYY-MM-DD - for folders)
    pub folder_date_format: String,
    
    /// Date format for FILE names ONLY (chrono format)
    /// Default: "%Y_%m_%d" (YYYY_MM_DD - for files, with underscores)
    pub file_date_format: String,
    
    /// Time format for {time} placeholder (chrono format)
    /// Default: "%H%M%S" (HHMMSS - 24-hour compact)
    pub time_format: String,
    
    /// File naming template
    /// Placeholders: {registry}, {seq}, {seq:03}, {date}, {time}, {datetime}
    /// {date} = uses file_date_format (underscores)
    /// {datetime} = {date}_{time} using file_date_format and time_format
    pub file_naming_template: String,
}

impl Default for RosterConfig {
    fn default() -> Self {
        RosterConfig {
            registry_types: vec!["citizens".to_string(), "workers".to_string()],
            max_selections: 4,
            max_people_per_file: 10,
            roster_length: 8,
            folder_date_format: "%Y-%m-%d".to_string(),     // YYYY-MM-DD for folders
            file_date_format: "%Y_%m_%d".to_string(),       // YYYY_MM_DD for files
            time_format: "%H%M%S".to_string(),              // HHMMSS
            file_naming_template: "{registry}_{seq:03}_{datetime}.md".to_string(),
        }
    }
}

// ──────────────────────────────────────────────────────────────
// Configuration Management
// ──────────────────────────────────────────────────────────────

lazy_static! {
    static ref CONFIG: RwLock<RosterConfig> = RwLock::new(RosterConfig::default());
}

#[derive(ThisError, Debug)]
pub enum ConfigError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("RON parsing error: {0}")]
    RonParse(String),
    
    #[error("Invalid configuration: {0}")]
    Invalid(String),
}

/// Get the configuration directory path
pub fn get_config_dir() -> Result<PathBuf, ConfigError> {
    // Try user config directory first
    if let Some(config_dir) = dirs::config_dir() {
        let roster_config = config_dir.join("roster");
        if roster_config.exists() {
            return Ok(roster_config);
        }
        // Create it if it doesn't exist
        std::fs::create_dir_all(&roster_config)?;
        return Ok(roster_config);
    }
    
    // Fallback to current directory
    let current_dir = std::env::current_dir()?;
    let local_config = current_dir.join(".roster");
    if !local_config.exists() {
        std::fs::create_dir_all(&local_config)?;
    }
    Ok(local_config)
}

/// Get the configuration file path
pub fn get_config_path() -> Result<PathBuf, ConfigError> {
    let config_dir = get_config_dir()?;
    Ok(config_dir.join("sortition.ron"))
}

/// Load configuration from file (or create default if not exists)
pub fn load_config() -> Result<(), ConfigError> {
    let config_path = get_config_path()?;
    
    if config_path.exists() {
        // Load from file
        let content = std::fs::read_to_string(&config_path)?;
        let config: RosterConfig = ron::from_str(&content)
            .map_err(|e| ConfigError::RonParse(e.to_string()))?;
        
        // Validate
        validate_config(&config)?;
        
        // Update global config
        let mut global_config = CONFIG.write().unwrap();
        *global_config = config;
        
        log::debug!("Configuration loaded from {:?}", config_path);
    } else {
        // Create default config file
        let config = RosterConfig::default();
        save_config(&config)?;
        log::debug!("Created default configuration at {:?}", config_path);
    }
    
    Ok(())
}

/// Save configuration to file
pub fn save_config(config: &RosterConfig) -> Result<(), ConfigError> {
    let config_path = get_config_path()?;
    
    // Validate before saving
    validate_config(config)?;
    
    let content = ron::ser::to_string_pretty(config, ron::ser::PrettyConfig::default())
        .map_err(|e| ConfigError::RonParse(e.to_string()))?;
    
    std::fs::write(&config_path, content)?;
    
    // Update global config
    let mut global_config = CONFIG.write().unwrap();
    *global_config = config.clone();
    
    log::debug!("Configuration saved to {:?}", config_path);
    Ok(())
}

/// Get current configuration
pub fn get_config() -> RosterConfig {
    let config = CONFIG.read().unwrap();
    config.clone()
}

/// Update configuration
pub fn update_config<F>(updater: F) -> Result<(), ConfigError> 
where
    F: FnOnce(&mut RosterConfig),
{
    let mut config = get_config();
    updater(&mut config);
    save_config(&config)
}

/// Validate configuration
fn validate_config(config: &RosterConfig) -> Result<(), ConfigError> {
    // Validate registry types
    if config.registry_types.is_empty() {
        return Err(ConfigError::Invalid("At least one registry type is required".to_string()));
    }
    
    // Check for duplicates
    let mut seen = HashSet::new();
    for reg_type in &config.registry_types {
        if seen.contains(reg_type) {
            return Err(ConfigError::Invalid(format!(
                "Duplicate registry type: '{}'",
                reg_type
            )));
        }
        seen.insert(reg_type);
    }
    
    // Validate max selections
    if config.max_selections == 0 {
        return Err(ConfigError::Invalid("max_selections must be at least 1".to_string()));
    }
    
    // Validate max people per file
    if config.max_people_per_file == 0 {
        return Err(ConfigError::Invalid("max_people_per_file must be at least 1".to_string()));
    }
    
    // Validate roster length
    if config.roster_length == 0 || config.roster_length > 20 {
        return Err(ConfigError::Invalid("roster_length must be between 1 and 20".to_string()));
    }
    
    Ok(())
}

// ──────────────────────────────────────────────────────────────
// Configuration-based Constants
// ──────────────────────────────────────────────────────────────

/// Check if a registry type is valid
pub fn is_valid_registry_type(registry_type: &str) -> bool {
    let config = CONFIG.read().unwrap();
    config.registry_types.contains(&registry_type.to_string())
}

/// Get all valid registry types
pub fn valid_registry_types() -> Vec<String> {
    let config = CONFIG.read().unwrap();
    config.registry_types.clone()
}

/// Get maximum selections per person
pub fn max_selections() -> u32 {
    let config = CONFIG.read().unwrap();
    config.max_selections
}

/// Get maximum people per file
pub fn max_people_per_file() -> usize {
    let config = CONFIG.read().unwrap();
    config.max_people_per_file
}

/// Get roster ID length
pub fn roster_length() -> usize {
    let config = CONFIG.read().unwrap();
    config.roster_length
}

// ──────────────────────────────────────────────────────────────
// Error Types
// ──────────────────────────────────────────────────────────────

#[derive(ThisError, Debug)]
pub enum RosterError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    
    #[error("Date parsing error: {0}")]
    DateParse(String),
    
    #[error("File not found: {0}")]
    FileNotFound(String),
    
    #[error("Invalid registry type: {0}")]
    InvalidRegistry(String),
    
    #[error("Duplicate roster found: {0}")]
    DuplicateRoster(String),
    
    #[error("No available people")]
    NoAvailablePeople,
    
    #[error("Directory not found: {0}")]
    DirectoryNotFound(String),
    
    #[error("Invalid data: {0}")]
    InvalidData(String),
}

// ──────────────────────────────────────────────────────────────
// Data Models
// ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Person {
    pub name: String,
    pub roster: String,
    pub birth_date: String,
    pub times_selected: u32,
}

impl PartialEq for Person {
    fn eq(&self, other: &Self) -> bool {
        self.roster == other.roster
    }
}

impl Eq for Person {}

impl Hash for Person {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.roster.hash(state);
    }
}

impl Person {
    pub fn parse_birth_date(&self) -> Result<NaiveDate, RosterError> {
        NaiveDate::parse_from_str(&self.birth_date, "%m/%d/%Y")
            .map_err(|e| RosterError::DateParse(format!("{}: {}", self.birth_date, e)))
    }
    
    pub fn is_available(&self) -> bool {
        let max = max_selections();
        self.times_selected < max
    }
    
    pub fn new(name: String, birth_date: String, roster: String) -> Self {
        Person {
            name,
            roster,
            birth_date,
            times_selected: 0,
        }
    }
}

// Person with source file information
#[derive(Debug, Clone)]
pub struct PersonWithSource {
    pub person: Person,
    pub file_path: PathBuf,
    pub date_folder: Option<String>,
}

// ──────────────────────────────────────────────────────────────
// File System Operations
// ──────────────────────────────────────────────────────────────

pub fn get_base_directory() -> Result<PathBuf, RosterError> {
    // Always use ~/Documents/md-data and create Documents if needed
    let home = dirs::home_dir()
        .ok_or_else(|| RosterError::DirectoryNotFound("Home directory not found".to_string()))?;
    
    let docs_dir = home.join("Documents");
    let md_data_dir = docs_dir.join("md-data");
    
    // Create Documents directory if it doesn't exist
    if !docs_dir.exists() {
        println!("📁 Creating Documents directory: {:?}", docs_dir);
        fs::create_dir_all(&docs_dir)?;
    }
    
    Ok(md_data_dir)
}

pub fn get_registry_directory(registry_type: &str) -> Result<PathBuf, RosterError> {
    if !is_valid_registry_type(registry_type) {
        return Err(RosterError::InvalidRegistry(registry_type.to_string()));
    }
    
    let base_dir = get_base_directory()?;
    let registry_dir = base_dir.join(registry_type);
    Ok(registry_dir)
}

pub fn ensure_directory_exists(path: &Path) -> Result<(), RosterError> {
    if !path.exists() {
        println!("📁 Creating directory: {:?}", path);
        fs::create_dir_all(path)?;
    }
    Ok(())
}

// ──────────────────────────────────────────────────────────────
// Roster Generation
// ──────────────────────────────────────────────────────────────

pub fn generate_roster(birth_date: NaiveDate, used_rosters: &HashSet<String>) -> String {
    let length = roster_length();
    let mut attempts = 0;
    let mut rng = rand::thread_rng();
    
    while attempts < 1000 {
        // Generate random roster
        let mut roster = String::with_capacity(length);
        for _ in 0..length {
            let letter = (rng.gen_range(0..26) as u8 + b'A') as char;
            roster.push(letter);
        }
        
        if !used_rosters.contains(&roster) {
            return roster;
        }
        
        attempts += 1;
    }
    
    // Fallback: use hash-based generation
    generate_roster_fallback(birth_date, used_rosters, length)
}

fn generate_roster_fallback(birth_date: NaiveDate, used_rosters: &HashSet<String>, length: usize) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    
    let date_str = birth_date.format("%Y%m%d").to_string();
    let mut hasher = DefaultHasher::new();
    date_str.hash(&mut hasher);
    let hash = hasher.finish();
    
    let base: u64 = 26;
    let mut roster = String::new();
    let mut current_hash = hash;
    
    for _ in 0..length {
        let digit = (current_hash % base) as u8;
        let ch = (b'A' + digit) as char;
        roster.push(ch);
        current_hash /= base;
    }
    
    // Ensure uniqueness
    let mut final_roster = roster;
    let mut suffix = 0;
    
    while used_rosters.contains(&final_roster) && suffix < 100 {
        let mut hasher = DefaultHasher::new();
        final_roster.hash(&mut hasher);
        suffix.hash(&mut hasher);
        let hash = hasher.finish();
        
        let mut new_roster = String::new();
        let mut current_hash = hash;
        
        for _ in 0..length {
            let digit = (current_hash % base) as u8;
            let ch = (b'A' + digit) as char;
            new_roster.push(ch);
            current_hash /= base;
        }
        
        final_roster = new_roster;
        suffix += 1;
    }
    
    final_roster
}

// ──────────────────────────────────────────────────────────────
// Formatted Name Generation
// ──────────────────────────────────────────────────────────────

/// Generate a formatted date folder name
pub fn generate_date_folder_name(registry_type: &str, date: NaiveDate) -> Result<String, RosterError> {
    let config = CONFIG.read().unwrap();
    
    let date_str = date.format(&config.folder_date_format).to_string();
    let folder_name = format!("{}-{}", registry_type, date_str);
    
    Ok(folder_name)
}

/// Generate a formatted filename
pub fn generate_formatted_filename(
    registry_type: &str, 
    sequence: u32
) -> Result<String, RosterError> {
    let config = CONFIG.read().unwrap();
    let now = Local::now();
    
    // Use file_date_format for files (underscores)
    let date_str = now.format(&config.file_date_format).to_string();
    let time_str = now.format(&config.time_format).to_string();
    let datetime_str = format!("{}_{}", date_str, time_str);
    
    let mut filename = config.file_naming_template
        .replace("{registry}", registry_type)
        .replace("{seq}", &sequence.to_string())
        .replace("{seq:03}", &format!("{:03}", sequence))
        .replace("{date}", &date_str)
        .replace("{time}", &time_str)
        .replace("{datetime}", &datetime_str);
    
    // Ensure it ends with .md
    if !filename.ends_with(".md") && !filename.ends_with(".markdown") {
        filename.push_str(".md");
    }
    
    Ok(filename)
}

// ──────────────────────────────────────────────────────────────
// Markdown File Operations
// ──────────────────────────────────────────────────────────────

pub fn parse_markdown_table(content: &str) -> Result<Vec<Person>, RosterError> {
    let lines: Vec<&str> = content.lines().collect();
    
    // Find the table start
    let mut table_start = 0;
    let mut found_table = false;
    
    for (i, line) in lines.iter().enumerate() {
        if line.starts_with("| Name | Roster |") {
            if let Some(next_line) = lines.get(i + 1) {
                if next_line.contains("---") || next_line.contains(":---") {
                    found_table = true;
                    table_start = i + 2;
                    break;
                }
            }
        }
    }
    
    if !found_table {
        return Ok(Vec::new());
    }
    
    // Parse table rows
    let mut people = Vec::new();
    for line in &lines[table_start..] {
        let trimmed = line.trim();
        if trimmed.is_empty() || !trimmed.starts_with('|') {
            continue;
        }
        
        let columns: Vec<&str> = trimmed.split('|')
            .skip(1)
            .filter(|s| !s.trim().is_empty())
            .map(|s| s.trim())
            .collect();
        
        if columns.len() >= 4 {
            let name = columns[0].trim_matches('*').trim().to_string();
            let roster = columns[1].trim_matches('*').trim().to_string();
            let birth_date = columns[2].to_string();
            let times_selected = columns[3].parse().unwrap_or(0);
            
            people.push(Person {
                name,
                roster,
                birth_date,
                times_selected,
            });
        }
    }
    
    Ok(people)
}

pub fn load_people_from_file(filepath: &Path) -> Result<Vec<Person>, RosterError> {
    if !filepath.exists() {
        return Err(RosterError::FileNotFound(filepath.display().to_string()));
    }
    
    let content = fs::read_to_string(filepath)?;
    parse_markdown_table(&content)
}

pub fn load_all_people_with_sources(
    registry_type: &str
) -> Result<Vec<PersonWithSource>, RosterError> {
    let registry_dir = get_registry_directory(registry_type)?;
    
    if !registry_dir.exists() {
        return Err(RosterError::DirectoryNotFound(registry_dir.display().to_string()));
    }
    
    let mut all_people = Vec::new();
    
    // Helper to load from a directory
    fn load_from_dir(
        dir: &Path, 
        date_folder: Option<String>
    ) -> Result<Vec<PersonWithSource>, RosterError> {
        let mut people_from_dir = Vec::new();
        
        if !dir.exists() {
            return Ok(people_from_dir);
        }
        
        // List markdown files
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == "md" || ext == "markdown" {
                        match load_people_from_file(&path) {
                            Ok(people) => {
                                for person in people {
                                    people_from_dir.push(PersonWithSource {
                                        person,
                                        file_path: path.clone(),
                                        date_folder: date_folder.clone(),
                                    });
                                }
                            }
                            Err(e) => {
                                println!("Warning: Error loading {}: {}", path.display(), e);
                            }
                        }
                    }
                }
            }
        }
        
        Ok(people_from_dir)
    }
    
    // Load from root directory
    all_people.extend(load_from_dir(&registry_dir, None)?);
    
    // Load from date folders
    for entry in fs::read_dir(&registry_dir)? {
        let entry = entry?;
        let path = entry.path();
        
        if path.is_dir() {
            if let Some(dir_name) = path.file_name() {
                if let Some(dir_name_str) = dir_name.to_str() {
                    if dir_name_str.starts_with(&format!("{}-", registry_type)) {
                        all_people.extend(load_from_dir(&path, Some(dir_name_str.to_string()))?);
                    }
                }
            }
        }
    }
    
    // Remove duplicates by roster ID
    let mut unique_people = Vec::new();
    let mut seen_rosters = HashSet::new();
    
    for person_with_source in all_people {
        if !seen_rosters.contains(&person_with_source.person.roster) {
            seen_rosters.insert(person_with_source.person.roster.clone());
            unique_people.push(person_with_source);
        }
    }
    
    Ok(unique_people)
}

pub fn load_all_people(registry_type: &str) -> Result<Vec<Person>, RosterError> {
    let people_with_sources = load_all_people_with_sources(registry_type)?;
    Ok(people_with_sources.into_iter()
        .map(|pws| pws.person)
        .collect())
}

// ──────────────────────────────────────────────────────────────
// File Writing Operations
// ──────────────────────────────────────────────────────────────

pub fn generate_markdown_content(
    people: &[Person],
    registry_type: &str,
    sequence: u32,
    date_folder: Option<&str>,
    total_unique_people: usize,
) -> String {
    let now = Local::now();
    let max_per_file = max_people_per_file();
    let max_selections = max_selections();
    
    let mut content = String::new();
    
    // Header
    content.push_str("# Roster Registry\n\n");
    content.push_str(&format!("*Registry Type: {}*\n", registry_type));
    content.push_str(&format!("*File Sequence: {}*\n", sequence));
    if let Some(folder) = date_folder {
        content.push_str(&format!("*Date Folder: {}*\n", folder));
    }
    content.push_str(&format!("*Generated: {}*\n", now.format("%B %d, %Y at %H:%M:%S")));
    content.push_str(&format!("*Entries in this file: {}*\n", people.len()));
    content.push_str(&format!("*Total unique entries in registry: {}*\n", total_unique_people));
    content.push_str("\n");
    
    // Table
    content.push_str("## Roster List\n\n");
    content.push_str("| Name | Roster | Birth Date | Times Selected |\n");
    content.push_str("| :--- | :----- | :--------- | :------------- |\n");
    
    for person in people {
        let times_selected_display = if person.times_selected >= max_selections {
            format!("**{}** (MAX)", person.times_selected)
        } else {
            format!("{}", person.times_selected)
        };
        
        content.push_str(&format!(
            "| {} | **{}** | {} | {} |\n", 
            person.name, 
            person.roster, 
            person.birth_date, 
            times_selected_display
        ));
    }
    
    content.push_str("\n");
    
    // Statistics
    content.push_str("## Statistics\n\n");
    content.push_str(&format!("- **People in this file:** {}\n", people.len()));
    content.push_str(&format!("- **Unique people in registry:** {}\n", total_unique_people));
    content.push_str(&format!("- **Maximum per file:** {}\n", max_per_file));
    content.push_str(&format!("- **Maximum selections per person:** {}\n", max_selections));
    
    if !people.is_empty() {
        let mut month_counts = [0; 12];
        for person in people {
            if let Ok(date) = person.parse_birth_date() {
                let month_index = (date.month() as usize) - 1;
                month_counts[month_index] += 1;
            }
        }
        
        if let Some((max_month, _)) = month_counts.iter().enumerate().max_by_key(|&(_, &count)| count) {
            let month_name = match max_month + 1 {
                1 => "January", 2 => "February", 3 => "March", 4 => "April",
                5 => "May", 6 => "June", 7 => "July", 8 => "August",
                9 => "September", 10 => "October", 11 => "November", 12 => "December",
                _ => "Unknown",
            };
            content.push_str(&format!("- **Most Common Birth Month in this file:** {}\n", month_name));
        }
    }
    
    content.push_str("\n");
    content.push_str("---\n");
    content.push_str("*Generated by Roster Generator v0.1.0*\n");
    content.push_str(&format!("*Configuration: {} types, {} length roster IDs*\n", 
        valid_registry_types().len(), roster_length()));
    
    content
}

pub fn write_markdown_file(
    filepath: &Path,
    content: &str,
) -> Result<(), RosterError> {
    let mut file = fs::File::create(filepath)?;
    write!(file, "{}", content)?;
    Ok(())
}

// ──────────────────────────────────────────────────────────────
// Selection Algorithms
// ──────────────────────────────────────────────────────────────

pub fn select_random_people(
    people: &mut [Person],
    count: usize,
) -> Result<Vec<Person>, RosterError> {
    let max_selections = max_selections();
    
    // Filter available people
    let available_indices: Vec<usize> = people.iter()
        .enumerate()
        .filter(|(_, p)| p.times_selected < max_selections)
        .map(|(i, _)| i)
        .collect();
    
    if available_indices.is_empty() {
        return Err(RosterError::NoAvailablePeople);
    }
    
    let actual_count = count.min(available_indices.len());
    
    // Create weighted selection pool
    let mut weighted_indices = Vec::new();
    for &idx in &available_indices {
        let remaining = max_selections - people[idx].times_selected;
        // More weight for people selected fewer times
        for _ in 0..remaining {
            weighted_indices.push(idx);
        }
    }
    
    let selection_pool = if !weighted_indices.is_empty() {
        weighted_indices
    } else {
        available_indices.clone()
    };
    
    let mut rng = rand::thread_rng();
    let mut selected_indices = Vec::new();
    let mut attempts = 0;
    let max_attempts = actual_count * 10;
    
    // Select unique people
    while selected_indices.len() < actual_count && attempts < max_attempts {
        if let Some(&random_idx) = selection_pool.choose(&mut rng) {
            if !selected_indices.contains(&random_idx) {
                selected_indices.push(random_idx);
            }
        }
        attempts += 1;
    }
    
    // Update selection counts and collect results
    let mut selected_people = Vec::new();
    for &idx in &selected_indices {
        people[idx].times_selected += 1;
        selected_people.push(people[idx].clone());
    }
    
    Ok(selected_people)
}

// ──────────────────────────────────────────────────────────────
// Date Folder Management
// ──────────────────────────────────────────────────────────────

pub fn list_date_folders(registry_type: &str) -> Result<Vec<String>, RosterError> {
    let registry_dir = get_registry_directory(registry_type)?;
    
    if !registry_dir.exists() {
        return Ok(Vec::new());
    }
    
    let mut folders = Vec::new();
    
    for entry in fs::read_dir(&registry_dir)? {
        let entry = entry?;
        let path = entry.path();
        
        if path.is_dir() {
            if let Some(dir_name) = path.file_name() {
                if let Some(dir_name_str) = dir_name.to_str() {
                    if dir_name_str.starts_with(&format!("{}-", registry_type)) {
                        folders.push(dir_name_str.to_string());
                    }
                }
            }
        }
    }
    
    // Sort by date (newest first)
    folders.sort_by(|a, b| b.cmp(a));
    
    Ok(folders)
}

pub fn create_date_folder(registry_type: &str, date: Option<NaiveDate>) -> Result<String, RosterError> {
    let today = date.unwrap_or_else(|| Local::now().date_naive());
    let folder_name = generate_date_folder_name(registry_type, today)?;
    
    let folder_path = get_registry_directory(registry_type)?.join(&folder_name);
    ensure_directory_exists(&folder_path)?;
    
    Ok(folder_name)
}

// ──────────────────────────────────────────────────────────────
// File Name Generation (Backward Compatible)
// ──────────────────────────────────────────────────────────────

pub fn generate_filename(registry_type: &str, sequence: u32) -> Result<String, RosterError> {
    generate_formatted_filename(registry_type, sequence)
}

pub fn extract_sequence_number(filename: &str) -> Option<u32> {
    let parts: Vec<&str> = filename.split('_').collect();
    if parts.len() >= 2 {
        parts[1].parse::<u32>().ok()
    } else {
        None
    }
}

// ──────────────────────────────────────────────────────────────
// Statistics
// ──────────────────────────────────────────────────────────────

pub struct RegistryStats {
    pub total_people: usize,
    pub available_people: usize,
    pub maxed_out_people: usize,
    pub unique_birth_dates: usize,
    pub most_common_birth_month: Option<&'static str>,
}

pub fn calculate_stats(people: &[Person]) -> RegistryStats {
    let max_selections = max_selections();
    let total_people = people.len();
    let available_people = people.iter().filter(|p| p.times_selected < max_selections).count();
    let maxed_out_people = total_people - available_people;
    
    let unique_birth_dates: HashSet<String> = people.iter()
        .map(|p| p.birth_date.clone())
        .collect();
    
    let mut month_counts = [0; 12];
    for person in people {
        if let Ok(date) = person.parse_birth_date() {
            let month_index = (date.month() as usize) - 1;
            month_counts[month_index] += 1;
        }
    }
    
    let most_common_birth_month = month_counts.iter()
        .enumerate()
        .max_by_key(|&(_, count)| count)
        .map(|(month, _)| match month + 1 {
            1 => "January", 2 => "February", 3 => "March", 4 => "April",
            5 => "May", 6 => "June", 7 => "July", 8 => "August",
            9 => "September", 10 => "October", 11 => "November", 12 => "December",
            _ => "Unknown",
        });
    
    RegistryStats {
        total_people,
        available_people,
        maxed_out_people,
        unique_birth_dates: unique_birth_dates.len(),
        most_common_birth_month,
    }
}

// ──────────────────────────────────────────────────────────────
// Utility Functions
// ──────────────────────────────────────────────────────────────

pub fn parse_date(date_str: &str) -> Result<NaiveDate, RosterError> {
    NaiveDate::parse_from_str(date_str, "%m/%d/%Y")
        .map_err(|e| RosterError::DateParse(format!("{}: {}", date_str, e)))
}

pub fn format_date(date: NaiveDate) -> String {
    date.format("%m/%d/%Y").to_string()
}

pub fn validate_name(name: &str) -> bool {
    !name.trim().is_empty()
}

pub fn validate_birth_date(date_str: &str) -> bool {
    parse_date(date_str).is_ok()
}

// ──────────────────────────────────────────────────────────────
// Registry State Management
// ──────────────────────────────────────────────────────────────

pub struct RegistryState {
    pub used_rosters: HashSet<String>,
    pub date_to_rosters: HashMap<NaiveDate, Vec<String>>,
    pub all_people: HashSet<Person>,
}

impl RegistryState {
    pub fn new() -> Self {
        RegistryState {
            used_rosters: HashSet::new(),
            date_to_rosters: HashMap::new(),
            all_people: HashSet::new(),
        }
    }
    
    pub fn add_person(&mut self, person: Person) -> Result<(), RosterError> {
        // Check for duplicate roster
        if self.used_rosters.contains(&person.roster) {
            return Err(RosterError::DuplicateRoster(person.roster));
        }
        
        // Parse birth date for tracking
        let birth_date = person.parse_birth_date()?;
        
        // Update tracking structures
        self.used_rosters.insert(person.roster.clone());
        self.date_to_rosters
            .entry(birth_date)
            .or_default()
            .push(person.roster.clone());
        
        // Add to all people
        self.all_people.insert(person);
        
        Ok(())
    }
    
    pub fn load_from_people(&mut self, people: &[Person]) -> Result<(), RosterError> {
        for person in people {
            self.add_person(person.clone())?;
        }
        Ok(())
    }
}