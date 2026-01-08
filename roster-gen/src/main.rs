use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::PathBuf;
use chrono::NaiveDate;
use chrono::prelude::*;
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;

#[derive(Debug, Deserialize, Serialize, Clone)]
struct Person {
    name: String,
    roster: String,
    birth_date: String,
    times_selected: u32,
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

struct RosterGenerator {
    used_rosters: HashSet<String>,
    date_to_rosters: HashMap<NaiveDate, Vec<String>>,
    people: Vec<Person>,  // People in current working batch
    all_people: HashSet<Person>,  // All unique people loaded from all files in this registry type
    base_dir: PathBuf,
    registry_type: String,
    current_file_index: u32,
    max_people_per_file: usize,
    recently_saved_files: Vec<String>,  // Track files saved in this session
    current_filename: Option<String>,   // Track which file we're currently working on
    date_folder: Option<String>,        // Selected date folder (e.g., "citizens-2024-12-20")
}

impl RosterGenerator {
    fn new(registry_type: &str) -> Result<Self, Box<dyn Error>> {
        // Try multiple methods to get Documents directory
        let documents_dir = get_documents_directory()?;
        
        println!("📍 Found Documents directory: {:?}", documents_dir);
        
        // Create the full path: ~/Documents/md-data/
        let base_dir = documents_dir.join("md-data");
        
        println!("📁 Will create data in: {:?}", base_dir);
        
        Ok(Self {
            used_rosters: HashSet::new(),
            date_to_rosters: HashMap::new(),
            people: Vec::new(),
            all_people: HashSet::new(),
            base_dir,
            registry_type: registry_type.to_string(),
            current_file_index: 1,
            max_people_per_file: 10,
            recently_saved_files: Vec::new(),
            current_filename: None,
            date_folder: None,
        })
    }

    fn ensure_directories(&self) -> Result<(), Box<dyn Error>> {
        println!("🔧 Checking if directory exists: {:?}", self.base_dir);
        
        // Create ~/Documents/md-data if it doesn't exist
        if !self.base_dir.exists() {
            println!("📁 Creating directory: {:?}", self.base_dir);
            fs::create_dir_all(&self.base_dir)?;
            println!("✅ Created: {:?}", self.base_dir);
        } else {
            println!("✅ Directory already exists: {:?}", self.base_dir);
        }
        
        // Create registry-specific subdirectory
        let registry_dir = self.get_registry_dir();
        println!("🔧 Checking if registry directory exists: {:?}", registry_dir);
        
        if !registry_dir.exists() {
            println!("📁 Creating registry directory: {:?}", registry_dir);
            fs::create_dir_all(&registry_dir)?;
            println!("✅ Created: {:?}", registry_dir);
        } else {
            println!("✅ Registry directory already exists: {:?}", registry_dir);
        }
        
        // Create date folder if one is selected
        if let Some(date_folder) = &self.date_folder {
            let date_dir = self.get_date_folder_dir();
            println!("🔧 Checking if date folder exists: {:?}", date_dir);
            
            if !date_dir.exists() {
                println!("📁 Creating date folder: {:?}", date_dir);
                fs::create_dir_all(&date_dir)?;
                println!("✅ Created: {:?}", date_dir);
            } else {
                println!("✅ Date folder already exists: {:?}", date_dir);
            }
        }
        
        Ok(())
    }

    fn get_registry_dir(&self) -> PathBuf {
        self.base_dir.join(&self.registry_type)
    }

    fn get_date_folder_dir(&self) -> PathBuf {
        if let Some(date_folder) = &self.date_folder {
            self.get_registry_dir().join(date_folder)
        } else {
            self.get_registry_dir()
        }
    }

    fn get_current_working_dir(&self) -> PathBuf {
        self.get_date_folder_dir()
    }

    fn list_date_folders(&self) -> Result<Vec<String>, Box<dyn Error>> {
        let registry_dir = self.get_registry_dir();
        
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
                        // Check if it matches the pattern: registry_type-YYYY-MM-DD
                        if dir_name_str.starts_with(&format!("{}-", self.registry_type)) {
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

    fn create_date_folder(&mut self, date: Option<NaiveDate>) -> Result<String, Box<dyn Error>> {
        let today = date.unwrap_or_else(|| Local::now().date_naive());
        let folder_name = format!("{}-{}", self.registry_type, today.format("%Y-%m-%d"));
        
        let folder_path = self.get_registry_dir().join(&folder_name);
        
        if !folder_path.exists() {
            println!("📁 Creating date folder: {}", folder_name);
            fs::create_dir_all(&folder_path)?;
            println!("✅ Created: {}", folder_name);
        } else {
            println!("📁 Using existing date folder: {}", folder_name);
        }
        
        self.date_folder = Some(folder_name.clone());
        
        Ok(folder_name)
    }

    fn load_all_existing_files_across_all_folders(&mut self) -> Result<(), Box<dyn Error>> {
        // Clear existing data
        self.used_rosters.clear();
        self.date_to_rosters.clear();
        self.all_people.clear();
        
        // Get all date folders for this registry type
        let date_folders = self.list_date_folders()?;
        
        // Also include the root registry directory (for files not in date folders)
        let mut all_dirs = vec![self.get_registry_dir()];
        for folder in &date_folders {
            all_dirs.push(self.get_registry_dir().join(folder));
        }
        
        let mut total_files = 0;
        
        for dir in all_dirs {
            if !dir.exists() {
                continue;
            }
            
            // List markdown files in this directory
            let mut files_in_dir = Vec::new();
            for entry in fs::read_dir(&dir)? {
                let entry = entry?;
                let path = entry.path();
                
                if path.is_file() {
                    if let Some(extension) = path.extension() {
                        if extension == "md" || extension == "markdown" {
                            if let Some(filename) = path.file_name() {
                                if let Some(filename_str) = filename.to_str() {
                                    files_in_dir.push(filename_str.to_string());
                                }
                            }
                        }
                    }
                }
            }
            
            for file in &files_in_dir {
                let filepath = dir.join(file);
                if filepath.exists() {
                    println!("📂 Loading data from: {:?}", filepath);
                    self.load_existing_markdown_from_path(&filepath)?;
                    total_files += 1;
                }
            }
        }
        
        println!("✓ Loaded data from {} file(s) across all folders", total_files);
        println!("  Unique people loaded: {}", self.all_people.len());
        Ok(())
    }

    fn load_existing_markdown_from_path(&mut self, filepath: &PathBuf) -> Result<(), Box<dyn Error>> {
        if !filepath.exists() {
            return Ok(());
        }
        
        let content = fs::read_to_string(filepath)?;
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
            return Ok(());
        }
        
        // Parse table rows
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
                
                // Parse birth date for internal tracking
                if let Ok(parsed_date) = NaiveDate::parse_from_str(&birth_date, "%m/%d/%Y") {
                    self.used_rosters.insert(roster.clone());
                    self.date_to_rosters
                        .entry(parsed_date)
                        .or_default()
                        .push(roster.clone());
                }
                
                let person = Person {
                    name,
                    roster,
                    birth_date,
                    times_selected,
                };
                
                // Only add if not already present (by roster)
                self.all_people.insert(person);
            }
        }
        
        Ok(())
    }

    fn load_file_for_editing(&mut self, filename: &str) -> Result<(), Box<dyn Error>> {
        // Clear current working batch
        self.people.clear();
        
        // Load data from the specific file into the current batch
        let filepath = self.get_current_working_dir().join(filename);
        
        if !filepath.exists() {
            return Err(format!("File not found: {}", filename).into());
        }
        
        let content = fs::read_to_string(&filepath)?;
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
            return Ok(());
        }
        
        // Parse table rows into current batch
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
                
                let person = Person {
                    name,
                    roster,
                    birth_date,
                    times_selected,
                };
                
                self.people.push(person);
            }
        }
        
        // Set the current filename
        self.current_filename = Some(filename.to_string());
        
        // Extract sequence number from filename
        let parts: Vec<&str> = filename.split('_').collect();
        if parts.len() >= 2 {
            if let Ok(seq_num) = parts[1].parse::<u32>() {
                self.current_file_index = seq_num;
            }
        }
        
        println!("📝 Loaded {} people from {}", self.people.len(), filename);
        Ok(())
    }

    fn find_incomplete_files(&self) -> Result<Vec<(String, usize)>, Box<dyn Error>> {
        let files = self.list_existing_markdown_files()?;
        let mut incomplete_files = Vec::new();
        
        for file in files {
            let filepath = self.get_current_working_dir().join(&file);
            if !filepath.exists() {
                continue;
            }
            
            let content = fs::read_to_string(&filepath)?;
            let lines: Vec<&str> = content.lines().collect();
            
            // Look for the "Entries in this file" line
            let mut count = 0;
            for line in &lines {
                if line.contains("Entries in this file:") {
                    let parts: Vec<&str> = line.split(':').collect();
                    if parts.len() >= 2 {
                        if let Ok(num) = parts[1].trim().trim_matches('*').parse::<usize>() {
                            count = num;
                            break;
                        }
                    }
                }
            }
            
            // Also check the table directly
            if count == 0 {
                // Find the table and count rows
                for (i, line) in lines.iter().enumerate() {
                    if line.starts_with("| Name | Roster |") {
                        if let Some(next_line) = lines.get(i + 1) {
                            if next_line.contains("---") || next_line.contains(":---") {
                                // Count table rows starting from i+2
                                for row_line in &lines[i+2..] {
                                    let trimmed = row_line.trim();
                                    if trimmed.is_empty() || !trimmed.starts_with('|') {
                                        break;
                                    }
                                    count += 1;
                                }
                                break;
                            }
                        }
                    }
                }
            }
            
            if count < self.max_people_per_file {
                incomplete_files.push((file, count));
            }
        }
        
        // Sort by sequence number
        incomplete_files.sort_by(|a, b| {
            let a_parts: Vec<&str> = a.0.split('_').collect();
            let b_parts: Vec<&str> = b.0.split('_').collect();
            
            if a_parts.len() >= 2 && b_parts.len() >= 2 {
                let a_num = a_parts[1].parse::<u32>().unwrap_or(0);
                let b_num = b_parts[1].parse::<u32>().unwrap_or(0);
                a_num.cmp(&b_num)
            } else {
                a.0.cmp(&b.0)
            }
        });
        
        Ok(incomplete_files)
    }

    fn get_next_file_number(&self) -> Result<u32, Box<dyn Error>> {
        let files = self.list_existing_markdown_files()?;
        
        // Extract numbers from filenames like citizens_001_2024_12_20_120000.md
        let mut max_number = 0;
        
        for file in files {
            let parts: Vec<&str> = file.split('_').collect();
            if parts.len() >= 2 {
                if let Ok(num) = parts[1].parse::<u32>() {
                    if num > max_number {
                        max_number = num;
                    }
                }
            }
        }
        
        Ok(max_number + 1)
    }

    fn generate_filename(&self, sequence: Option<u32>) -> String {
        let now = Local::now();
        let datetime = now.format("%Y_%m_%d_%H%M%S").to_string();
        
        let seq_num = sequence.unwrap_or(self.current_file_index);
        
        format!("{}_{:03}_{}.md", self.registry_type, seq_num, datetime)
    }

    fn list_existing_markdown_files(&self) -> Result<Vec<String>, Box<dyn Error>> {
        let working_dir = self.get_current_working_dir();
        
        if !working_dir.exists() {
            return Ok(Vec::new());
        }
        
        let mut files = Vec::new();
        
        for entry in fs::read_dir(&working_dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_file() {
                if let Some(extension) = path.extension() {
                    if extension == "md" || extension == "markdown" {
                        if let Some(filename) = path.file_name() {
                            if let Some(filename_str) = filename.to_str() {
                                files.push(filename_str.to_string());
                            }
                        }
                    }
                }
            }
        }
        
        // Sort by sequence number then by timestamp
        files.sort_by(|a, b| {
            let a_parts: Vec<&str> = a.split('_').collect();
            let b_parts: Vec<&str> = b.split('_').collect();
            
            if a_parts.len() >= 2 && b_parts.len() >= 2 {
                let a_num = a_parts[1].parse::<u32>().unwrap_or(0);
                let b_num = b_parts[1].parse::<u32>().unwrap_or(0);
                a_num.cmp(&b_num)
            } else {
                a.cmp(b)
            }
        });
        
        Ok(files)
    }

    fn generate_roster(&mut self, birth_date: NaiveDate) -> String {
        if let Some(existing_rosters) = self.date_to_rosters.get(&birth_date) {
            if !existing_rosters.is_empty() {
                let mut attempts = 0;
                
                while attempts < 1000 {
                    let mut hasher = DefaultHasher::new();
                    birth_date.hash(&mut hasher);
                    attempts.hash(&mut hasher);
                    let hash = hasher.finish();
                    
                    let base: u64 = 26;
                    let mut roster = String::new();
                    let mut current_hash = hash;
                    
                    for _ in 0..8 {
                        let digit = (current_hash % base) as u8;
                        let ch = (b'A' + digit) as char;
                        roster.push(ch);
                        current_hash /= base;
                    }
                    
                    if !self.used_rosters.contains(&roster) && !existing_rosters.contains(&roster) {
                        return roster;
                    }
                    
                    attempts += 1;
                }
                
                // Fallback
                let mut roster_num = self.used_rosters.len() as u64;
                let base: u64 = 26;
                
                loop {
                    let mut current_num = roster_num;
                    let mut roster = String::new();
                    
                    for _ in 0..8 {
                        let digit = (current_num % base) as u8;
                        let ch = (b'A' + digit) as char;
                        roster.push(ch);
                        current_num /= base;
                    }
                    
                    let roster = roster.chars().rev().collect::<String>();
                    
                    if !self.used_rosters.contains(&roster) && !existing_rosters.contains(&roster) {
                        return roster;
                    }
                    roster_num += 1;
                }
            }
        }

        // First person with this birth date
        let mut hasher = DefaultHasher::new();
        birth_date.hash(&mut hasher);
        let hash = hasher.finish();
        
        let base: u64 = 26;
        let mut roster = String::new();
        let mut current_hash = hash;
        
        for _ in 0..8 {
            let digit = (current_hash % base) as u8;
            let ch = (b'A' + digit) as char;
            roster.push(ch);
            current_hash /= base;
        }
        
        let mut attempts = 0;
        let mut final_roster = roster;
        
        while self.used_rosters.contains(&final_roster) && attempts < 100 {
            let mut hasher = DefaultHasher::new();
            birth_date.hash(&mut hasher);
            attempts.hash(&mut hasher);
            let hash = hasher.finish();
            
            let mut new_roster = String::new();
            let mut current_hash = hash;
            
            for _ in 0..8 {
                let digit = (current_hash % base) as u8;
                let ch = (b'A' + digit) as char;
                new_roster.push(ch);
                current_hash /= base;
            }
            
            final_roster = new_roster;
            attempts += 1;
        }

        final_roster
    }

    fn add_person(&mut self, name: &str, birth_date_str: &str) -> Result<Person, Box<dyn Error>> {
        let birth_date = NaiveDate::parse_from_str(birth_date_str, "%m/%d/%Y")?;
        
        let roster = self.generate_roster(birth_date);
        
        let new_person = Person {
            name: name.to_string(),
            roster: roster.clone(),
            birth_date: birth_date_str.to_string(),
            times_selected: 0,
        };

        // Track roster for collision prevention
        self.used_rosters.insert(roster.clone());
        self.date_to_rosters
            .entry(birth_date)
            .or_default()
            .push(roster.clone());
        
        // Add to current batch
        self.people.push(new_person.clone());
        
        // Add to all_people (HashSet ensures uniqueness)
        self.all_people.insert(new_person.clone());

        Ok(new_person)
    }

    fn should_create_new_file(&self) -> bool {
        self.people.len() >= self.max_people_per_file
    }

    fn save_current_batch(&mut self) -> Result<String, Box<dyn Error>> {
        if self.people.is_empty() {
            return Err("No people to save".into());
        }
        
        // Determine filename - either use current one or generate new
        let filename = if let Some(ref current_file) = self.current_filename {
            // We're overwriting an existing file
            current_file.clone()
        } else {
            // Creating a new file
            self.generate_filename(Some(self.current_file_index))
        };
        
        self.save_to_markdown(&filename, &self.people)?;
        
        // Track this file for reloading
        self.recently_saved_files.push(filename.clone());
        
        // IMPORTANT: Don't reload the file here since we already have the people in memory
        // Clearing and reloading would cause duplicates
        
        // If this was a new file, increment the counter
        if self.current_filename.is_none() {
            self.current_file_index += 1;
        }
        
        // Clear current batch but keep people in all_people
        self.people.clear();
        
        // Clear current filename since we're done with this file
        self.current_filename = None;
        
        Ok(filename)
    }

    fn save_to_markdown(&self, filename: &str, people_to_save: &[Person]) -> Result<(), Box<dyn Error>> {
        let filepath = self.get_current_working_dir().join(filename);
        
        let overwriting = filepath.exists();
        
        println!("💾 Saving batch of {} people to: {}", people_to_save.len(), filename);
        if overwriting {
            println!("   (Overwriting existing file)");
        }
        
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&filepath)?;
        
        // Determine sequence number from filename
        let mut seq_num = self.current_file_index;
        let parts: Vec<&str> = filename.split('_').collect();
        if parts.len() >= 2 {
            if let Ok(num) = parts[1].parse::<u32>() {
                seq_num = num;
            }
        }
        
        // Header
        writeln!(file, "# Roster Registry")?;
        writeln!(file)?;
        writeln!(file, "*Registry Type: {}*", self.registry_type)?;
        writeln!(file, "*File Sequence: {}*", seq_num)?;
        if let Some(date_folder) = &self.date_folder {
            writeln!(file, "*Date Folder: {}*", date_folder)?;
        }
        writeln!(file, "*Generated: {}*", Local::now().format("%B %d, %Y at %H:%M:%S"))?;
        if overwriting {
            writeln!(file, "*Updated: {}*", Local::now().format("%B %d, %Y at %H:%M:%S"))?;
        }
        writeln!(file, "*Entries in this file: {}*", people_to_save.len())?;
        writeln!(file, "*Total unique entries in registry: {}*", self.all_people.len())?;
        writeln!(file)?;
        
        // Table
        writeln!(file, "## Roster List")?;
        writeln!(file)?;
        writeln!(file, "| Name | Roster | Birth Date | Times Selected |")?;
        writeln!(file, "| :--- | :----- | :--------- | :------------- |")?;
        
        for person in people_to_save {
            writeln!(
                file, 
                "| {} | **{}** | {} | {} |", 
                person.name, 
                person.roster, 
                person.birth_date, 
                person.times_selected
            )?;
        }
        
        writeln!(file)?;
        
        // Statistics
        writeln!(file, "## Statistics")?;
        writeln!(file)?;
        writeln!(file, "- **People in this file:** {}", people_to_save.len())?;
        writeln!(file, "- **Unique people in registry:** {}", self.all_people.len())?;
        writeln!(file, "- **Unique Birth Dates in registry:** {}", self.date_to_rosters.len())?;
        
        if !people_to_save.is_empty() {
            let mut month_counts = [0; 12];
            for person in people_to_save {
                if let Ok(date) = NaiveDate::parse_from_str(&person.birth_date, "%m/%d/%Y") {
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
                writeln!(file, "- **Most Common Birth Month in this file:** {}", month_name)?;
            }
        }
        
        writeln!(file)?;
        writeln!(file, "---")?;
        writeln!(file, "*Generated by Roster Generator v0.1.0*")?;
        writeln!(file, "*File Location: {:?}*", filepath)?;
        writeln!(file, "*Limit: {} people per file*", self.max_people_per_file)?;
        if overwriting {
            writeln!(file, "*Note: This file was updated*")?;
        }
        
        println!("✅ Batch saved to: {:?}", filepath);
        Ok(())
    }
}

// Helper function to get Documents directory
fn get_documents_directory() -> Result<PathBuf, Box<dyn Error>> {
    // Method 1: Use dirs crate
    if let Some(docs) = dirs::document_dir() {
        return Ok(docs);
    }
    
    // Method 2: Try environment variables
    #[cfg(target_os = "windows")]
    {
        if let Ok(userprofile) = std::env::var("USERPROFILE") {
            let docs = PathBuf::from(userprofile).join("Documents");
            if docs.exists() {
                return Ok(docs);
            }
        }
    }
    
    #[cfg(target_os = "macos")]
    {
        if let Ok(home) = std::env::var("HOME") {
            let docs = PathBuf::from(home).join("Documents");
            if docs.exists() {
                return Ok(docs);
            }
        }
    }
    
    #[cfg(target_os = "linux")]
    {
        if let Ok(home) = std::env::var("HOME") {
            let docs = PathBuf::from(home).join("Documents");
            if docs.exists() {
                return Ok(docs);
            }
        }
    }
    
    // Method 3: Fallback to current directory
    std::env::current_dir()
        .map_err(|e| format!("Could not get current directory: {}", e).into())
}

fn main() -> Result<(), Box<dyn Error>> {
    println!("╔══════════════════════════════════════╗");
    println!("║      Markdown Roster Generator       ║");
    println!("║         (10 people per file)         ║");
    println!("╚══════════════════════════════════════╝");
    println!();
    println!("Choose registry type:");
    println!("1. Citizens registry");
    println!("2. Workers registry");
    
    let mut choice = String::new();
    io::stdin().read_line(&mut choice)?;
    
    let registry_type = match choice.trim() {
        "1" => "citizens",
        "2" => "workers",
        _ => {
            println!("⚠️  Invalid choice. Defaulting to citizens registry.");
            "citizens"
        }
    };
    
    println!("\n🔧 Initializing {} registry...", registry_type);
    
    let mut generator = match RosterGenerator::new(registry_type) {
        Ok(g) => {
            println!("✓ Registry system initialized");
            g
        },
        Err(e) => {
            println!("❌ Error initializing registry: {}", e);
            println!("   Creating directory in current location...");
            
            // Fallback to project directory
            let current_dir = std::env::current_dir()?;
            let base_dir = current_dir.join("md-data");
            
            println!("📁 Using project directory: {:?}", base_dir);
            
            RosterGenerator {
                used_rosters: HashSet::new(),
                date_to_rosters: HashMap::new(),
                people: Vec::new(),
                all_people: HashSet::new(),
                base_dir,
                registry_type: registry_type.to_string(),
                current_file_index: 1,
                max_people_per_file: 10,
                recently_saved_files: Vec::new(),
                current_filename: None,
                date_folder: None,
            }
        }
    };
    
    // Ensure base directories exist
    if let Err(e) = generator.ensure_directories() {
        println!("❌ Error creating directories: {}", e);
        println!("Please check permissions or manually create:");
        println!("  {:?}", generator.base_dir);
        println!("  {:?}", generator.get_registry_dir());
        return Err(e);
    }
    
    println!("📂 Base directory: {:?}", generator.get_registry_dir());
    
    // Date folder selection
    println!("\n📅 Date Folder Selection:");
    println!("1. Create new date folder (today's date)");
    println!("2. Select existing date folder");
    println!("3. Don't use a date folder (save directly in registry folder)");
    
    let mut date_choice = String::new();
    io::stdin().read_line(&mut date_choice)?;
    
    match date_choice.trim() {
        "1" => {
            // Create new date folder for today
            match generator.create_date_folder(None) {
                Ok(folder_name) => {
                    println!("✅ Created/using date folder: {}", folder_name);
                }
                Err(e) => {
                    println!("❌ Error creating date folder: {}", e);
                    println!("   Will save directly in registry folder.");
                }
            }
        }
        "2" => {
            // List existing date folders
            match generator.list_date_folders() {
                Ok(folders) => {
                    if folders.is_empty() {
                        println!("⚠️  No existing date folders found.");
                        println!("   Creating new one for today...");
                        generator.create_date_folder(None)?;
                    } else {
                        println!("\n📋 Existing date folders:");
                        for (i, folder) in folders.iter().enumerate() {
                            println!("   {}. {}", i + 1, folder);
                        }
                        
                        println!("\nEnter the number of the folder to use:");
                        let mut folder_choice = String::new();
                        io::stdin().read_line(&mut folder_choice)?;
                        
                        if let Ok(choice_num) = folder_choice.trim().parse::<usize>() {
                            if choice_num >= 1 && choice_num <= folders.len() {
                                let folder_name = &folders[choice_num - 1];
                                generator.date_folder = Some(folder_name.clone());
                                println!("✅ Selected date folder: {}", folder_name);
                            } else {
                                println!("⚠️  Invalid choice. Creating new folder.");
                                generator.create_date_folder(None)?;
                            }
                        } else {
                            println!("⚠️  Invalid input. Creating new folder.");
                            generator.create_date_folder(None)?;
                        }
                    }
                }
                Err(e) => {
                    println!("❌ Error listing date folders: {}", e);
                    println!("   Creating new folder.");
                    generator.create_date_folder(None)?;
                }
            }
        }
        "3" => {
            println!("📁 Will save directly in registry folder.");
        }
        _ => {
            println!("⚠️  Invalid choice. Creating new date folder.");
            generator.create_date_folder(None)?;
        }
    }
    
    // Now ensure the selected date folder exists
    if let Err(e) = generator.ensure_directories() {
        println!("❌ Error creating date folder: {}", e);
        return Err(e);
    }
    
    println!("📂 Working in directory: {:?}", generator.get_current_working_dir());
    
    // Load all existing data from ALL FOLDERS for this registry type
    println!("\n📥 Loading all existing {} data from ALL folders...", registry_type);
    match generator.load_all_existing_files_across_all_folders() {
        Ok(_) => {
            println!("✓ Loaded {} unique roster(s) from all folders", generator.all_people.len());
        }
        Err(e) => {
            println!("⚠️  Could not load existing data: {}", e);
            println!("   Starting with empty registry.");
        }
    }
    
    // Look for incomplete files (only in the selected folder)
    match generator.find_incomplete_files() {
        Ok(incomplete_files) => {
            if !incomplete_files.is_empty() {
                println!("\n📋 Found {} incomplete file(s) in current folder:", incomplete_files.len());
                for (i, (file, _count)) in incomplete_files.iter().enumerate() {
                    println!("   {}. {} (incomplete)", i + 1, file);
                }
                
                println!("\nChoose an option:");
                println!("1. Continue with the most recent incomplete file");
                println!("2. Start a new file");
                println!("3. Select a specific file from the list");
                
                let mut file_choice = String::new();
                io::stdin().read_line(&mut file_choice)?;
                
                match file_choice.trim() {
                    "1" => {
                        // Use the most recent incomplete file
                        if let Some((filename, _count)) = incomplete_files.last() {
                            println!("\n📝 Continuing with: {}", filename);
                            if let Err(e) = generator.load_file_for_editing(filename) {
                                println!("❌ Error loading file: {}", e);
                                println!("   Starting new file instead.");
                            }
                        }
                    }
                    "2" => {
                        println!("\n📄 Starting new file...");
                        match generator.get_next_file_number() {
                            Ok(num) => generator.current_file_index = num,
                            Err(_) => generator.current_file_index = 1,
                        }
                    }
                    "3" => {
                        println!("\nEnter the number of the file to continue:");
                        let mut num_choice = String::new();
                        io::stdin().read_line(&mut num_choice)?;
                        
                        if let Ok(choice_num) = num_choice.trim().parse::<usize>() {
                            if choice_num >= 1 && choice_num <= incomplete_files.len() {
                                let (filename, _) = &incomplete_files[choice_num - 1];
                                println!("\n📝 Continuing with: {}", filename);
                                if let Err(e) = generator.load_file_for_editing(filename) {
                                    println!("❌ Error loading file: {}", e);
                                    println!("   Starting new file instead.");
                                }
                            } else {
                                println!("⚠️  Invalid choice. Starting new file.");
                                match generator.get_next_file_number() {
                                    Ok(num) => generator.current_file_index = num,
                                    Err(_) => generator.current_file_index = 1,
                                }
                            }
                        } else {
                            println!("⚠️  Invalid input. Starting new file.");
                            match generator.get_next_file_number() {
                                Ok(num) => generator.current_file_index = num,
                                Err(_) => generator.current_file_index = 1,
                            }
                        }
                    }
                    _ => {
                        println!("⚠️  Invalid choice. Starting new file.");
                        match generator.get_next_file_number() {
                            Ok(num) => generator.current_file_index = num,
                            Err(_) => generator.current_file_index = 1,
                        }
                    }
                }
            } else {
                println!("\n📋 No incomplete files found in current folder.");
                println!("📄 Starting new file...");
                match generator.get_next_file_number() {
                    Ok(num) => generator.current_file_index = num,
                    Err(_) => generator.current_file_index = 1,
                }
            }
        }
        Err(e) => {
            println!("⚠️  Error checking for incomplete files: {}", e);
            println!("📄 Starting new file...");
            match generator.get_next_file_number() {
                Ok(num) => generator.current_file_index = num,
                Err(_) => generator.current_file_index = 1,
            }
        }
    }
    
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Enter person details (type 'save' to finish):");
    if let Some(ref filename) = generator.current_filename {
        println!("📄 Current file: {}", filename);
    } else {
        println!("📄 Next file will be: {}", generator.generate_filename(None));
    }
    if let Some(ref date_folder) = generator.date_folder {
        println!("📁 Date folder: {}", date_folder);
    }
    println!("📊 Unique people in ALL {} folders: {}", registry_type, generator.all_people.len());
    println!("🔍 Checking against {} known rosters", generator.used_rosters.len());
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    let mut files_created = Vec::new();
    
    loop {
        // Check if we need to create a new file
        if generator.should_create_new_file() {
            println!("\n📦 Current batch is full ({} people)", generator.max_people_per_file);
            println!("   Saving current file...");
            
            match generator.save_current_batch() {
                Ok(filename) => {
                    files_created.push(filename.clone());
                    println!("✅ Saved file: {}", filename);
                    println!("📝 Starting new batch...");
                    println!("📄 Next file will be: {}", generator.generate_filename(None));
                    
                    // Show updated totals
                    println!("📊 Registry now has {} unique people across all folders", generator.all_people.len());
                    println!("   (Checking against {} known rosters)", generator.used_rosters.len());
                }
                Err(e) => {
                    println!("❌ Error saving file: {}", e);
                }
            }
        }
        
        println!("\n👤 Name (or 'save' to finish):");
        println!("   [Current batch: {}/{} people]", 
                generator.people.len(), generator.max_people_per_file);
        println!("   [Unique people in ALL {} folders: {}]", registry_type, generator.all_people.len());
        
        let mut name = String::new();
        io::stdin().read_line(&mut name)?;
        let name = name.trim();
        
        if name.eq_ignore_ascii_case("save") {
            break;
        }
        
        if name.is_empty() {
            println!("❌ Name cannot be empty.");
            continue;
        }
        
        println!("📅 Birth Date (MM/DD/YYYY):");
        let mut birth_date = String::new();
        io::stdin().read_line(&mut birth_date)?;
        let birth_date = birth_date.trim();
        
        match generator.add_person(name, birth_date) {
            Ok(person) => {
                println!("\n✅ Successfully added:");
                println!("   ┌─────────────────────┐");
                println!("   │ Name: {:16} │", person.name);
                println!("   │ Roster: {:14} │", person.roster);
                println!("   │ Birth Date: {:11} │", person.birth_date);
                println!("   │ Batch: {:3}/{}         │", 
                        generator.people.len(), generator.max_people_per_file);
                println!("   │ Unique total: {:7} │", generator.all_people.len());
                println!("   └─────────────────────┘");
            }
            Err(e) => {
                println!("❌ Error: {}", e);
                println!("   Please try again with valid date format (MM/DD/YYYY)");
            }
        }
    }
    
    // Save any remaining people
    if !generator.people.is_empty() {
        println!("\n💾 Saving current batch of {} people...", generator.people.len());
        match generator.save_current_batch() {
            Ok(filename) => {
                files_created.push(filename.clone());
                println!("✅ Saved file: {}", filename);
            }
            Err(e) => {
                println!("❌ Error saving final batch: {}", e);
            }
        }
    }
    
    // Don't do final reload - it would create duplicates
    println!("\n📊 Final counts:");
    println!("  Unique people in ALL {} folders: {}", registry_type, generator.all_people.len());
    println!("  Known rosters for collision checking: {}", generator.used_rosters.len());
    
    // Summary
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("✅ Registry update complete!");
    println!("📂 Working directory: {:?}", generator.get_current_working_dir());
    
    if !files_created.is_empty() {
        println!("\n📄 Files saved/updated:");
        for file in &files_created {
            println!("   • {}", file);
        }
    } else {
        println!("\n📄 No changes saved (no data entered)");
    }
    
    println!("\n📊 Summary:");
    println!("   • Unique people in ALL {} folders: {}", registry_type, generator.all_people.len());
    println!("   • Unique birth dates: {}", generator.date_to_rosters.len());
    println!("   • Known rosters for collision checking: {}", generator.used_rosters.len());
    println!("   • People per file limit: {}", generator.max_people_per_file);
    
    // Show the next available file number
    let next_file_num = match generator.get_next_file_number() {
        Ok(num) => num,
        Err(_) => 1,
    };
    println!("   • Next file number: {}", next_file_num);
    
    // List all files in the current working directory
    match generator.list_existing_markdown_files() {
        Ok(files) => {
            println!("   • Total files in current directory: {}", files.len());
            
            // Show incomplete files for next time
            match generator.find_incomplete_files() {
                Ok(incomplete) => {
                    if !incomplete.is_empty() {
                        println!("\n📋 Incomplete files in current folder for next session:");
                        for (file, count) in incomplete {
                            println!("   • {} ({} of {} people)", file, count, generator.max_people_per_file);
                        }
                    }
                }
                Err(_) => {}
            }
        }
        Err(_) => {}
    }
    
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    Ok(())
}