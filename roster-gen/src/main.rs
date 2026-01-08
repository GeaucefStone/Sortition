use roster_core::*;
use std::collections::{HashMap, HashSet};
use std::io::{self, Write};
use std::path::PathBuf;
use chrono::NaiveDate;

struct RosterGenerator {
    state: RegistryState,
    people: Vec<Person>,  // Current working batch
    base_dir: PathBuf,
    registry_type: String,
    current_file_index: u32,
    recently_saved_files: Vec<String>,
    current_filename: Option<String>,
    date_folder: Option<String>,
}

impl RosterGenerator {
    fn new(registry_type: &str) -> Result<Self, Box<dyn std::error::Error>> {
        // Load configuration first
        load_config()?;
        
        let base_dir = get_base_directory()?;
        
        println!("📍 Found base directory: {:?}", base_dir);
        
        Ok(Self {
            state: RegistryState::new(),
            people: Vec::new(),
            base_dir,
            registry_type: registry_type.to_string(),
            current_file_index: 1,
            recently_saved_files: Vec::new(),
            current_filename: None,
            date_folder: None,
        })
    }

    fn ensure_directories(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("🔧 Checking if directory exists: {:?}", self.base_dir);
        
        // Create base directory if it doesn't exist
        ensure_directory_exists(&self.base_dir)?;
        println!("✅ Base directory: {:?}", self.base_dir);
        
        // Create registry-specific subdirectory
        let registry_dir = get_registry_directory(&self.registry_type)?;
        println!("🔧 Checking if registry directory exists: {:?}", registry_dir);
        ensure_directory_exists(&registry_dir)?;
        println!("✅ Registry directory: {:?}", registry_dir);
        
        // Create date folder if one is selected
        if let Some(date_folder) = &self.date_folder {
            let date_dir = self.get_date_folder_dir()?;
            println!("🔧 Checking if date folder exists: {:?}", date_dir);
            ensure_directory_exists(&date_dir)?;
            println!("✅ Date folder: {:?}", date_dir);
        }
        
        Ok(())
    }

    fn get_date_folder_dir(&self) -> Result<PathBuf, Box<dyn std::error::Error>> {
        if let Some(date_folder) = &self.date_folder {
            let registry_dir = get_registry_directory(&self.registry_type)?;
            Ok(registry_dir.join(date_folder))
        } else {
            Ok(get_registry_directory(&self.registry_type)?)
        }
    }

    fn get_current_working_dir(&self) -> Result<PathBuf, Box<dyn std::error::Error>> {
        self.get_date_folder_dir()
    }

    fn create_date_folder(&mut self, date: Option<NaiveDate>) -> Result<String, Box<dyn std::error::Error>> {
        let folder_name = create_date_folder(&self.registry_type, date)?;
        self.date_folder = Some(folder_name.clone());
        Ok(folder_name)
    }

    fn load_all_existing_files(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("📥 Loading all existing data...");
        
        // Load all people from all files
        let all_people = load_all_people(&self.registry_type)?;
        
        // Update state
        self.state = RegistryState::new();
        self.state.load_from_people(&all_people)?;
        
        println!("✓ Loaded {} unique people from all folders", self.state.all_people.len());
        Ok(())
    }

    fn load_file_for_editing(&mut self, filename: &str) -> Result<(), Box<dyn std::error::Error>> {
        // Clear current working batch
        self.people.clear();
        
        // Load data from the specific file
        let filepath = self.get_current_working_dir()?.join(filename);
        let people_from_file = load_people_from_file(&filepath)?;
        
        // Add to current batch
        self.people.extend(people_from_file);
        
        // Set the current filename
        self.current_filename = Some(filename.to_string());
        
        // Extract sequence number from filename
        if let Some(seq_num) = extract_sequence_number(filename) {
            self.current_file_index = seq_num;
        }
        
        println!("📝 Loaded {} people from {}", self.people.len(), filename);
        Ok(())
    }

    fn find_incomplete_files(&self) -> Result<Vec<(String, usize)>, Box<dyn std::error::Error>> {
        let working_dir = self.get_current_working_dir()?;
        
        if !working_dir.exists() {
            return Ok(Vec::new());
        }
        
        let mut incomplete_files = Vec::new();
        
        // List files in directory
        for entry in std::fs::read_dir(&working_dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == "md" || ext == "markdown" {
                        if let Some(filename) = path.file_name() {
                            if let Some(filename_str) = filename.to_str() {
                                // Load file to count people
                                match load_people_from_file(&path) {
                                    Ok(people) => {
                                        if people.len() < max_people_per_file() {
                                            incomplete_files.push((filename_str.to_string(), people.len()));
                                        }
                                    }
                                    Err(_) => continue,
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // Sort by sequence number
        incomplete_files.sort_by(|a, b| {
            let a_num = extract_sequence_number(&a.0).unwrap_or(0);
            let b_num = extract_sequence_number(&b.0).unwrap_or(0);
            a_num.cmp(&b_num)
        });
        
        Ok(incomplete_files)
    }

    fn get_next_file_number(&self) -> Result<u32, Box<dyn std::error::Error>> {
        let working_dir = self.get_current_working_dir()?;
        
        if !working_dir.exists() {
            return Ok(1);
        }
        
        let mut max_number = 0;
        
        for entry in std::fs::read_dir(&working_dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_file() {
                if let Some(filename) = path.file_name() {
                    if let Some(filename_str) = filename.to_str() {
                        if let Some(num) = extract_sequence_number(filename_str) {
                            if num > max_number {
                                max_number = num;
                            }
                        }
                    }
                }
            }
        }
        
        Ok(max_number + 1)
    }

    fn add_person(&mut self, name: &str, birth_date_str: &str) -> Result<Person, Box<dyn std::error::Error>> {
        let birth_date = parse_date(birth_date_str)?;
        
        // Generate roster using existing rosters for collision prevention
        let roster = generate_roster(birth_date, &self.state.used_rosters);
        
        let new_person = Person::new(
            name.to_string(),
            birth_date_str.to_string(),
            roster.clone(),
        );

        // Add to state for tracking
        self.state.add_person(new_person.clone())?;
        
        // Add to current batch
        self.people.push(new_person.clone());

        Ok(new_person)
    }

    fn should_create_new_file(&self) -> bool {
        self.people.len() >= max_people_per_file()
    }

    fn save_current_batch(&mut self) -> Result<String, Box<dyn std::error::Error>> {
        if self.people.is_empty() {
            return Err("No people to save".into());
        }
        
        // Determine filename
        let filename = if let Some(ref current_file) = self.current_filename {
            current_file.clone()
        } else {
            generate_filename(&self.registry_type, self.current_file_index)?
        };
        
        self.save_to_markdown(&filename)?;
        
        // Track this file
        self.recently_saved_files.push(filename.clone());
        
        // If this was a new file, increment the counter
        if self.current_filename.is_none() {
            self.current_file_index += 1;
        }
        
        // Clear current batch
        self.people.clear();
        self.current_filename = None;
        
        Ok(filename)
    }

    fn save_to_markdown(&self, filename: &str) -> Result<(), Box<dyn std::error::Error>> {
        let filepath = self.get_current_working_dir()?.join(filename);
        let overwriting = filepath.exists();
        
        println!("💾 Saving batch of {} people to: {}", self.people.len(), filename);
        if overwriting {
            println!("   (Overwriting existing file)");
        }
        
        // Get sequence number
        let seq_num = extract_sequence_number(filename).unwrap_or(self.current_file_index);
        
        // Generate content using shared library
        let content = generate_markdown_content(
            &self.people,
            &self.registry_type,
            seq_num,
            self.date_folder.as_deref(),
            self.state.all_people.len(),
        );
        
        // Write file
        write_markdown_file(&filepath, &content)?;
        
        println!("✅ Batch saved to: {:?}", filepath);
        Ok(())
    }

    fn list_existing_markdown_files(&self) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let working_dir = self.get_current_working_dir()?;
        
        if !working_dir.exists() {
            return Ok(Vec::new());
        }
        
        let mut files = Vec::new();
        
        for entry in std::fs::read_dir(&working_dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == "md" || ext == "markdown" {
                        if let Some(filename) = path.file_name() {
                            if let Some(filename_str) = filename.to_str() {
                                files.push(filename_str.to_string());
                            }
                        }
                    }
                }
            }
        }
        
        // Sort by sequence number
        files.sort_by(|a, b| {
            let a_num = extract_sequence_number(a).unwrap_or(0);
            let b_num = extract_sequence_number(b).unwrap_or(0);
            a_num.cmp(&b_num)
        });
        
        Ok(files)
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("╔══════════════════════════════════════╗");
    println!("║      Markdown Roster Generator       ║");
    println!("║         Config: sortition.ron        ║");
    println!("╚══════════════════════════════════════╝");
    println!();
    
    // Load configuration
    println!("📋 Loading configuration...");
    match load_config() {
        Ok(_) => {
            let config = get_config();
            println!("✅ Loaded configuration:");
            println!("   • Registry types: {}", config.registry_types.join(", "));
            println!("   • Max selections: {}", config.max_selections);
            println!("   • Max per file: {}", config.max_people_per_file);
            println!("   • Roster length: {}", config.roster_length);
        }
        Err(e) => {
            println!("⚠️  Configuration error: {}", e);
            println!("   Using default configuration.");
        }
    }
    
    println!("\nChoose registry type:");
    let valid_types = valid_registry_types();
    for (i, reg_type) in valid_types.iter().enumerate() {
        println!("  {}. {}", i + 1, reg_type);
    }
    
    let mut choice = String::new();
    io::stdin().read_line(&mut choice)?;
    
    let registry_type = match choice.trim().parse::<usize>() {
        Ok(num) if num >= 1 && num <= valid_types.len() => {
            &valid_types[num - 1]
        }
        _ => {
            println!("⚠️  Invalid choice. Defaulting to first registry type.");
            &valid_types[0]
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
            return Err(e);
        }
    };
    
    // Ensure base directories exist
    if let Err(e) = generator.ensure_directories() {
        println!("❌ Error creating directories: {}", e);
        return Err(e);
    }
    
    // Date folder selection
    println!("\n📅 Date Folder Selection:");
    println!("1. Create new date folder (today's date)");
    println!("2. Select existing date folder");
    println!("3. Don't use a date folder (save directly in registry folder)");
    
    let mut date_choice = String::new();
    io::stdin().read_line(&mut date_choice)?;
    
    match date_choice.trim() {
        "1" => {
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
            match list_date_folders(registry_type) {
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
    
    // Load all existing data
    println!("\n📥 Loading all existing {} data...", registry_type);
    match generator.load_all_existing_files() {
        Ok(_) => {
            println!("✓ Loaded {} unique roster(s)", generator.state.all_people.len());
        }
        Err(e) => {
            println!("⚠️  Could not load existing data: {}", e);
            println!("   Starting with empty registry.");
        }
    }
    
    // Look for incomplete files
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
        println!("📄 Next file will be: {}", 
                 generate_filename(registry_type, generator.current_file_index)?);
    }
    if let Some(ref date_folder) = generator.date_folder {
        println!("📁 Date folder: {}", date_folder);
    }
    println!("📊 Unique people in ALL {} folders: {}", registry_type, generator.state.all_people.len());
    println!("🔍 Checking against {} known rosters", generator.state.used_rosters.len());
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    let mut files_created = Vec::new();
    
    loop {
        // Check if we need to create a new file
        if generator.should_create_new_file() {
            println!("\n📦 Current batch is full ({} people)", max_people_per_file());
            println!("   Saving current file...");
            
            match generator.save_current_batch() {
                Ok(filename) => {
                    files_created.push(filename.clone());
                    println!("✅ Saved file: {}", filename);
                    println!("📝 Starting new batch...");
                    println!("📄 Next file will be: {}", 
                             generate_filename(registry_type, generator.current_file_index)?);
                    
                    // Show updated totals
                    println!("📊 Registry now has {} unique people across all folders", 
                             generator.state.all_people.len());
                    println!("   (Checking against {} known rosters)", 
                             generator.state.used_rosters.len());
                }
                Err(e) => {
                    println!("❌ Error saving file: {}", e);
                }
            }
        }
        
        println!("\n👤 Name (or 'save' to finish):");
        println!("   [Current batch: {}/{} people]", 
                generator.people.len(), max_people_per_file());
        println!("   [Unique people in ALL {} folders: {}]", 
                 registry_type, generator.state.all_people.len());
        
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
                        generator.people.len(), max_people_per_file());
                println!("   │ Unique total: {:7} │", generator.state.all_people.len());
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
    
    println!("\n📊 Final counts:");
    println!("  Unique people in ALL {} folders: {}", 
             registry_type, generator.state.all_people.len());
    println!("  Known rosters for collision checking: {}", 
             generator.state.used_rosters.len());
    
    // Summary
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("✅ Registry update complete!");
    
    if !files_created.is_empty() {
        println!("\n📄 Files saved/updated:");
        for file in &files_created {
            println!("   • {}", file);
        }
    } else {
        println!("\n📄 No changes saved (no data entered)");
    }
    
    println!("\n📊 Summary:");
    println!("   • Unique people in ALL {} folders: {}", 
             registry_type, generator.state.all_people.len());
    println!("   • People per file limit: {}", max_people_per_file());
    
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
                            println!("   • {} ({} of {} people)", file, count, max_people_per_file());
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