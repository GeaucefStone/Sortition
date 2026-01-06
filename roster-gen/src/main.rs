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

struct RosterGenerator {
    used_rosters: HashSet<String>,
    date_to_rosters: HashMap<NaiveDate, Vec<String>>,
    people: Vec<Person>,
    base_dir: PathBuf,
    registry_type: String, // "citizens" or "workers"
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
            base_dir,
            registry_type: registry_type.to_string(),
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
        
        Ok(())
    }

    fn get_registry_dir(&self) -> PathBuf {
        self.base_dir.join(&self.registry_type)
    }

    fn load_existing_markdown(&mut self, filename: &str) -> Result<(), Box<dyn Error>> {
        // Ensure directories exist first
        self.ensure_directories()?;
        
        let filepath = self.get_registry_dir().join(filename);
        
        println!("📂 Looking for file: {:?}", filepath);
        
        if !filepath.exists() {
            println!("⚠️  File does not exist: {:?}", filepath);
            return Ok(());
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
            println!("No valid table found in {}", filename);
            return Ok(());
        }
        
        // Parse table rows
        for line in &lines[table_start..] {
            let trimmed = line.trim();
            if trimmed.is_empty() || !trimmed.starts_with('|') {
                continue;
            }
            
            let columns: Vec<&str> = trimmed.split('|')
                .skip(1) // Skip empty before first |
                .filter(|s| !s.trim().is_empty())
                .map(|s| s.trim())
                .collect();
            
            if columns.len() >= 4 {
                // Remove markdown formatting if present
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
                
                self.people.push(person);
            }
        }
        
        println!("Loaded {} existing rosters from {}", self.people.len(), filename);
        Ok(())
    }

    fn generate_filename(&self) -> String {
        let now = Local::now();
        let datetime = now.format("%Y_%m_%d_%H%M%S").to_string();
        format!("{}_{}.md", self.registry_type, datetime)
    }

    fn list_existing_markdown_files(&self) -> Result<Vec<String>, Box<dyn Error>> {
        // Ensure directories exist first
        self.ensure_directories()?;
        
        let mut files = Vec::new();
        let registry_dir = self.get_registry_dir();
        
        println!("📂 Scanning directory: {:?}", registry_dir);
        
        for entry in fs::read_dir(&registry_dir)? {
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
        
        files.sort();
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

        self.used_rosters.insert(roster.clone());
        self.date_to_rosters
            .entry(birth_date)
            .or_default()
            .push(roster.clone());
        
        self.people.push(new_person.clone());

        Ok(new_person)
    }

    fn save_to_markdown(&self, filename: &str) -> Result<(), Box<dyn Error>> {
        // Ensure directories exist before saving
        println!("🔧 Ensuring directories exist...");
        self.ensure_directories()?;
        
        let filepath = self.get_registry_dir().join(filename);
        
        println!("💾 Attempting to save to: {:?}", filepath);
        
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&filepath)?;
        
        // Header
        writeln!(file, "# Roster Registry")?;
        writeln!(file)?;
        writeln!(file, "*Registry Type: {}*", self.registry_type)?;
        writeln!(file, "*Generated: {}*", Local::now().format("%B %d, %Y at %H:%M:%S"))?;
        writeln!(file, "*Total Entries: {}*", self.people.len())?;
        writeln!(file)?;
        
        // Table
        writeln!(file, "## Roster List")?;
        writeln!(file)?;
        writeln!(file, "| Name | Roster | Birth Date | Times Selected |")?;
        writeln!(file, "| :--- | :----- | :--------- | :------------- |")?;
        
        for person in &self.people {
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
        writeln!(file, "- **Total People:** {}", self.people.len())?;
        writeln!(file, "- **Unique Birth Dates:** {}", self.date_to_rosters.len())?;
        
        if !self.people.is_empty() {
            let mut month_counts = [0; 12];
            for person in &self.people {
                if let Ok(date) = NaiveDate::parse_from_str(&person.birth_date, "%m/%d/%Y") {
                    // month() returns 1-12, subtract 1 for array index
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
                writeln!(file, "- **Most Common Birth Month:** {}", month_name)?;
            }
        }
        
        writeln!(file)?;
        writeln!(file, "---")?;
        writeln!(file, "*Generated by Roster Generator v0.1.0*")?;
        writeln!(file, "*File Location: {:?}*", filepath)?;
        
        println!("✅ File saved successfully to: {:?}", filepath);
        Ok(())
    }
}

// Helper function to get Documents directory
fn get_documents_directory() -> Result<PathBuf, Box<dyn Error>> {
    // Method 1: Use dirs crate
    if let Some(docs) = dirs::document_dir() {
        println!("📁 Using dirs::document_dir(): {:?}", docs);
        return Ok(docs);
    }
    
    // Method 2: Try environment variables
    #[cfg(target_os = "windows")]
    {
        if let Ok(userprofile) = std::env::var("USERPROFILE") {
            let docs = PathBuf::from(userprofile).join("Documents");
            println!("📁 Using USERPROFILE/Documents: {:?}", docs);
            if docs.exists() {
                return Ok(docs);
            }
        }
    }
    
    #[cfg(target_os = "macos")]
    {
        if let Ok(home) = std::env::var("HOME") {
            let docs = PathBuf::from(home).join("Documents");
            println!("📁 Using HOME/Documents: {:?}", docs);
            if docs.exists() {
                return Ok(docs);
            }
        }
    }
    
    #[cfg(target_os = "linux")]
    {
        if let Ok(home) = std::env::var("HOME") {
            let docs = PathBuf::from(home).join("Documents");
            println!("📁 Using HOME/Documents: {:?}", docs);
            if docs.exists() {
                return Ok(docs);
            }
        }
    }
    
    // Method 3: Fallback to current directory
    println!("⚠️  Could not find Documents directory, using current directory");
    std::env::current_dir()
        .map_err(|e| format!("Could not get current directory: {}", e).into())
}

fn main() -> Result<(), Box<dyn Error>> {
    println!("╔══════════════════════════════════════╗");
    println!("║      Markdown Roster Generator       ║");
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
                base_dir,
                registry_type: registry_type.to_string(),
            }
        }
    };
    
    // Ensure directories exist from the start
    if let Err(e) = generator.ensure_directories() {
        println!("❌ Error creating directories: {}", e);
        println!("Please check permissions or manually create:");
        println!("  {:?}", generator.base_dir);
        println!("  {:?}", generator.get_registry_dir());
        return Err(e);
    }
    
    println!("📂 Working in directory: {:?}", generator.get_registry_dir());
    
    println!("\nChoose option:");
    println!("1. Create new {} registry", registry_type);
    println!("2. Load existing {} registry", registry_type);
    
    let mut choice = String::new();
    io::stdin().read_line(&mut choice)?;
    
    let filename = match choice.trim() {
        "1" => {
            let filename = generator.generate_filename();
            println!("\n📄 Creating new {} registry: {}", registry_type, filename);
            filename
        }
        "2" => {
            match generator.list_existing_markdown_files() {
                Ok(files) => {
                    if files.is_empty() {
                        println!("\n⚠️  No existing {} registries found.", registry_type);
                        let filename = generator.generate_filename();
                        println!("   Creating new registry: {}", filename);
                        filename
                    } else {
                        println!("\n📂 Existing {} registries:", registry_type);
                        for (i, file) in files.iter().enumerate() {
                            println!("   {}. {}", i + 1, file);
                        }
                        
                        println!("\nEnter the number of the registry to load:");
                        let mut file_choice = String::new();
                        io::stdin().read_line(&mut file_choice)?;
                        
                        if let Ok(choice_num) = file_choice.trim().parse::<usize>() {
                            if choice_num >= 1 && choice_num <= files.len() {
                                let filename = files[choice_num - 1].clone();
                                if let Err(e) = generator.load_existing_markdown(&filename) {
                                    println!("⚠️  Error loading file: {}", e);
                                    println!("   Creating new registry instead.");
                                    generator.generate_filename()
                                } else {
                                    println!("✓ Loaded registry: {}", filename);
                                    filename
                                }
                            } else {
                                println!("⚠️  Invalid choice. Creating new registry.");
                                generator.generate_filename()
                            }
                        } else {
                            println!("⚠️  Invalid input. Creating new registry.");
                            generator.generate_filename()
                        }
                    }
                }
                Err(e) => {
                    println!("⚠️  Error listing files: {}", e);
                    println!("   Creating new registry.");
                    generator.generate_filename()
                }
            }
        }
        _ => {
            println!("⚠️  Invalid choice. Creating new registry.");
            generator.generate_filename()
        }
    };
    
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Enter person details (type 'save' to finish):");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    loop {
        println!("\n👤 Name (or 'save' to finish):");
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
                println!("   └─────────────────────┘");
            }
            Err(e) => {
                println!("❌ Error: {}", e);
                println!("   Please try again with valid date format (MM/DD/YYYY)");
            }
        }
    }
    
    println!("\n💾 Saving registry...");
    match generator.save_to_markdown(&filename) {
        Ok(_) => {
            println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            println!("✅ Registry saved successfully!");
            println!("📂 Directory: {:?}", generator.get_registry_dir());
            println!("📄 File: {}", filename);
            println!("👥 Total entries: {}", generator.people.len());
            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            
            // Show the exact path for easy access
            let full_path = generator.get_registry_dir().join(&filename);
            println!("\n📍 Full path: {:?}", full_path);
        }
        Err(e) => {
            println!("\n❌ Error saving file: {}", e);
            println!("\nTroubleshooting:");
            println!("1. Check if you have write permissions to: {:?}", generator.base_dir);
            println!("2. Try creating the directory manually:");
            println!("   mkdir -p {:?}", generator.get_registry_dir());
            println!("3. Or run the program from a different location");
        }
    }
    
    Ok(())
}