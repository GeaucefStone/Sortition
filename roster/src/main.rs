use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::io;
use std::fs::OpenOptions;
use std::path::Path;
use csv::{Reader, Writer};
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
}

impl RosterGenerator {
    fn new() -> Self {
        Self {
            used_rosters: HashSet::new(),
            date_to_rosters: HashMap::new(),
        }
    }

    fn load_existing_data(&mut self, filename: &str) -> Result<(), Box<dyn Error>> {
        if !Path::new(filename).exists() {
            return Ok(());
        }
        
        let mut rdr = Reader::from_path(filename)?;

        for record in rdr.deserialize() {
            let person: Person = record?;
            
            // Parse birth date
            let birth_date = NaiveDate::parse_from_str(&person.birth_date, "%m/%d/%Y")?;
            
            // Store the existing roster
            self.used_rosters.insert(person.roster.clone());
            
            // Add to date_to_rosters mapping
            self.date_to_rosters
                .entry(birth_date)
                .or_insert_with(Vec::new)
                .push(person.roster.clone());
        }

        println!("Loaded {} existing rosters from {}", self.used_rosters.len(), filename);
        Ok(())
    }

    fn generate_filename(&self, file_type: &str) -> String {
        let now = Local::now();
        let datetime = now.format("%Y_%m_%d_%H%M%S").to_string();
        format!("{}_{}.csv", file_type, datetime)
    }

    fn list_existing_files(&self) -> Result<Vec<String>, Box<dyn Error>> {
        let mut files = Vec::new();
        
        for entry in std::fs::read_dir(".")? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_file() {
                if let Some(extension) = path.extension() {
                    if extension == "csv" {
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
        // Check if we already have rosters for this birth date
        if let Some(existing_rosters) = self.date_to_rosters.get(&birth_date) {
            // If this is not the first person with this birth date,
            // we need to generate a different roster
            if !existing_rosters.is_empty() {
                let mut attempts = 0;
                
                // Keep generating until we find a unique roster for this date
                while attempts < 1000 {
                    // Generate roster with salt (attempt number) to get different results
                    let mut hasher = DefaultHasher::new();
                    birth_date.hash(&mut hasher);
                    attempts.hash(&mut hasher); // Use attempt count as salt
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
                    
                    // Check if this roster is unique globally AND not already used for this date
                    if !self.used_rosters.contains(&roster) && !existing_rosters.contains(&roster) {
                        return roster;
                    }
                    
                    attempts += 1;
                }
                
                // Fallback: sequential approach if hash collisions persist
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

        // First person with this birth date - generate initial roster
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
        
        // Ensure uniqueness
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

    fn add_person_to_csv(&mut self, filename: &str, name: &str, birth_date_str: &str) -> Result<Person, Box<dyn Error>> {
        // Parse birth date
        let birth_date = NaiveDate::parse_from_str(birth_date_str, "%m/%d/%Y")?;
        
        // Generate roster
        let roster = self.generate_roster(birth_date);
        
        let new_person = Person {
            name: name.to_string(),
            roster: roster.clone(),
            birth_date: birth_date_str.to_string(),
            times_selected: 0,
        };

        // Update our mappings
        self.used_rosters.insert(roster.clone());
        self.date_to_rosters
            .entry(birth_date)
            .or_insert_with(Vec::new)
            .push(roster.clone());

        // Check if file exists and is empty to determine if we need header
        let needs_header = !Path::new(filename).exists() || 
                          std::fs::metadata(filename)?.len() == 0;

        // Append to CSV file
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(filename)?;
        
        let mut wtr = Writer::from_writer(file);
        
        // Write header if needed
        if needs_header {
            wtr.write_record(&["Name", "Roster", "Birth Date", "Times Selected"])?;
        }
        
        wtr.write_record(&[&new_person.name, &new_person.roster, &new_person.birth_date, &new_person.times_selected.to_string()])?;
        wtr.flush()?;

        Ok(new_person)
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut generator = RosterGenerator::new();
    let filename;
    
    println!("Roster Generator");
    println!("Choose option:");
    println!("1. Create new citizens file (citizens_yyyy_mm_dd_HHMMSS.csv)");
    println!("2. Create new workers file (workers_yyyy_mm_dd_HHMMSS.csv)");
    println!("3. Resume existing file");
    
    let mut choice = String::new();
    io::stdin().read_line(&mut choice)?;
    
    match choice.trim() {
        "1" => {
            filename = generator.generate_filename("citizens");
            println!("Using filename: {}", filename);
        }
        "2" => {
            filename = generator.generate_filename("workers");
            println!("Using filename: {}", filename);
        }
        "3" => {
            // List existing CSV files
            let files = generator.list_existing_files()?;
            
            if files.is_empty() {
                println!("No existing CSV files found. Creating new citizens file.");
                filename = generator.generate_filename("citizens");
            } else {
                println!("Existing CSV files:");
                for (i, file) in files.iter().enumerate() {
                    println!("{}. {}", i + 1, file);
                }
                
                println!("Enter the number of the file to resume:");
                let mut file_choice = String::new();
                io::stdin().read_line(&mut file_choice)?;
                
                if let Ok(choice_num) = file_choice.trim().parse::<usize>() {
                    if choice_num >= 1 && choice_num <= files.len() {
                        filename = files[choice_num - 1].clone();
                        println!("Resuming file: {}", filename);
                    } else {
                        println!("Invalid choice. Creating new citizens file.");
                        filename = generator.generate_filename("citizens");
                    }
                } else {
                    println!("Invalid input. Creating new citizens file.");
                    filename = generator.generate_filename("citizens");
                }
            }
        }
        _ => {
            println!("Invalid choice. Defaulting to new citizens file.");
            filename = generator.generate_filename("citizens");
        }
    }
    
    // Load existing data from CSV to learn used rosters (if file exists)
    if let Err(_e) = generator.load_existing_data(&filename) {
        println!("Starting with new file: {}", filename);
    } else {
        println!("Resumed existing file: {} ({} rosters loaded)", filename, generator.used_rosters.len());
    }
    
    println!("\nEnter person details (or 'quit' to exit):");
    
    loop {
        println!("\nEnter name:");
        let mut name = String::new();
        io::stdin().read_line(&mut name)?;
        let name = name.trim();
        
        if name.eq_ignore_ascii_case("quit") {
            break;
        }
        
        if name.is_empty() {
            println!("Name cannot be empty. Please try again.");
            continue;
        }
        
        println!("Enter birth date (MM/DD/YYYY):");
        let mut birth_date = String::new();
        io::stdin().read_line(&mut birth_date)?;
        let birth_date = birth_date.trim();
        
        match generator.add_person_to_csv(&filename, name, birth_date) {
            Ok(person) => {
                println!("\n✓ Successfully added to CSV:");
                println!("Name: {}", person.name);
                println!("Roster: {}", person.roster);
                println!("Birth Date: {}", person.birth_date);
                println!("Times Selected: {}", person.times_selected);
                println!("\nCSV format: {},{},{},{}", person.name, person.roster, person.birth_date, person.times_selected);
            }
            Err(e) => {
                println!("Error: {}", e);
                println!("Please try again with valid date format (MM/DD/YYYY)");
            }
        }
    }
    
    println!("Goodbye! File saved as: {}", filename);
    Ok(())
}