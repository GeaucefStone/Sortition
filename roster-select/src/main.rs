use roster_core::*;
use std::collections::HashMap;
use std::io::{self, Write};
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("╔══════════════════════════════════════╗");
    println!("║     Markdown Roster Selector         ║");
    println!("║   (Scans ALL files & date folders)   ║");
    println!("║      Config: sortition.ron           ║");
    println!("╚══════════════════════════════════════╝");
    
    // Load configuration first
    println!("\n📋 Loading configuration...");
    match load_config() {
        Ok(_) => {
            let config = get_config();
            println!("✅ Loaded configuration:");
            println!("   • Registry types: {}", config.registry_types.join(", "));
            println!("   • Max selections per person: {}", config.max_selections);
            println!("   • File limit: {} people per file", config.max_people_per_file);
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
    
    println!("\n🔧 Initializing {} registry selector...", registry_type);
    
    // Check if directory exists
    let registry_dir = get_registry_directory(registry_type)?;
    if !registry_dir.exists() {
        println!("\n❌ ERROR: Registry directory does not exist: {:?}", registry_dir);
        println!("💡 You need to run the Roster Generator first!");
        println!("   Expected location: {:?}", registry_dir);
        return Ok(());
    }
    
    // Load ALL people with source information
    println!("\n📥 Loading ALL people from {} registry...", registry_type);
    let mut all_people_with_sources = match load_all_people_with_sources(registry_type) {
        Ok(people) => {
            if people.is_empty() {
                println!("❌ No people found in registry.");
                println!("💡 Run the Roster Generator to create roster files first.");
                return Ok(());
            }
            println!("✅ Successfully loaded {} unique people", people.len());
            people
        },
        Err(e) => {
            println!("❌ Error loading people: {}", e);
            return Ok(());
        }
    };
    
    // Show initial status
    let stats = calculate_stats(
        &all_people_with_sources.iter()
            .map(|pws| pws.person.clone())
            .collect::<Vec<_>>()
    );
    let max_selections = max_selections();
    
    println!("\n📊 Initial Status:");
    println!("   Total People: {}", stats.total_people);
    println!("   Available: {} people (selected less than {} times)", 
             stats.available_people, max_selections);
    println!("   Maxed Out: {} people (selected {} times)", 
             stats.maxed_out_people, max_selections);
    
    // Main selection loop
    loop {
        println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("Options:");
        println!("  1. View all people (all files/folders)");
        println!("  2. View available people only");
        println!("  3. Select random people");
        println!("  4. Reset ALL selection counts to 0");
        println!("  5. Reload data from files");
        println!("  6. Show configuration");
        println!("  7. Exit");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("Enter your choice (1-7):");
        
        let mut option = String::new();
        io::stdin().read_line(&mut option)?;
        
        match option.trim() {
            "1" => display_people(&all_people_with_sources, false),
            "2" => {
                let available_count = all_people_with_sources.iter()
                    .filter(|pws| pws.person.is_available())
                    .count();
                if available_count == 0 {
                    println!("❌ No available people!");
                    println!("   All {} people have reached the maximum of {} selections.", 
                             all_people_with_sources.len(), max_selections);
                } else {
                    display_people(&all_people_with_sources, true);
                }
            }
            "3" => {
                let available_count = all_people_with_sources.iter()
                    .filter(|pws| pws.person.is_available())
                    .count();
                
                if available_count == 0 {
                    println!("❌ No available people to select!");
                    println!("   All {} people have reached the maximum of {} selections.", 
                             all_people_with_sources.len(), max_selections);
                    println!("   Use option 4 to reset counts if needed.");
                    continue;
                }
                
                println!("How many people to select? (1-{})", available_count);
                let mut count_str = String::new();
                io::stdin().read_line(&mut count_str)?;
                
                match count_str.trim().parse::<usize>() {
                    Ok(count) if count >= 1 && count <= available_count => {
                        println!("\n🎲 Selecting {} person(s)...", count);
                        
                        // Extract people for selection
                        let mut people: Vec<Person> = all_people_with_sources.iter()
                            .map(|pws| pws.person.clone())
                            .collect();
                        
                        match select_random_people(&mut people, count) {
                            Ok(selected) => {
                                if selected.is_empty() {
                                    println!("❌ No people were selected.");
                                } else {
                                    println!("\n✅ Selected {} person(s):", selected.len());
                                    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
                                    
                                    // Update the source data structures
                                    let selected_rosters: Vec<String> = selected.iter()
                                        .map(|p| p.roster.clone())
                                        .collect();
                                    
                                    for person_with_source in &mut all_people_with_sources {
                                        if selected_rosters.contains(&person_with_source.person.roster) {
                                            // Find the updated person
                                            if let Some(updated_person) = people.iter()
                                                .find(|p| p.roster == person_with_source.person.roster)
                                            {
                                                person_with_source.person = updated_person.clone();
                                            }
                                        }
                                    }
                                    
                                    // Display selection
                                    for person in &selected {
                                        println!("• {} - {} ({})", 
                                                 person.name, person.roster, person.birth_date);
                                        println!("  Now selected: {}/{} times", 
                                                 person.times_selected, max_selections);
                                        
                                        if person.times_selected >= max_selections {
                                            println!("  ⚠️  MAXED OUT - Will not be selected again");
                                        }
                                        println!();
                                    }
                                    
                                    // Update the source files
                                    println!("💾 Updating source files...");
                                    let file_groups = group_people_by_file(&all_people_with_sources);
                                    let mut files_updated = 0;
                                    
                                    for (filepath, people) in file_groups {
                                        if let Err(e) = update_file_with_people(&filepath, &people) {
                                            println!("⚠️  Error updating {:?}: {}", 
                                                     filepath.file_name().unwrap_or_default(), e);
                                        } else {
                                            files_updated += 1;
                                        }
                                    }
                                    
                                    // Show updated stats
                                    let updated_stats = calculate_stats(&people);
                                    println!("\n📊 Updated Status:");
                                    println!("   Total People: {}", updated_stats.total_people);
                                    println!("   Available: {} people", updated_stats.available_people);
                                    println!("   Maxed Out: {} people", updated_stats.maxed_out_people);
                                    println!("   Files updated: {}", files_updated);
                                }
                            }
                            Err(e) => {
                                println!("❌ Error during selection: {}", e);
                            }
                        }
                    }
                    Ok(_) => println!("❌ Please enter a number between 1 and {}", available_count),
                    Err(_) => println!("❌ Invalid number."),
                }
            }
            "4" => {
                println!("⚠️  Are you sure you want to reset ALL selection counts to 0? (yes/no)");
                println!("   This will update ALL files in ALL date folders.");
                println!("   Affects {} people across all {} folders.", 
                         all_people_with_sources.len(), registry_type);
                
                let mut confirm = String::new();
                io::stdin().read_line(&mut confirm)?;
                
                if confirm.trim().eq_ignore_ascii_case("yes") {
                    println!("🔄 Resetting ALL selection counts...");
                    
                    // Reset in memory
                    for person_with_source in &mut all_people_with_sources {
                        person_with_source.person.times_selected = 0;
                    }
                    
                    // Group by file and update each file
                    let file_groups = group_people_by_file(&all_people_with_sources);
                    let mut files_updated = 0;
                    
                    for (filepath, people) in file_groups {
                        if let Err(e) = update_file_with_people(&filepath, &people) {
                            println!("⚠️  Error updating {:?}: {}", 
                                     filepath.file_name().unwrap_or_default(), e);
                        } else {
                            files_updated += 1;
                        }
                    }
                    
                    println!("✅ Reset complete!");
                    println!("   Updated {} file(s)", files_updated);
                    println!("   All {} people now have 0 selections", all_people_with_sources.len());
                } else {
                    println!("Reset cancelled.");
                }
            }
            "5" => {
                println!("🔄 Reloading data from files...");
                match load_all_people_with_sources(registry_type) {
                    Ok(people) => {
                        all_people_with_sources = people;
                        let stats = calculate_stats(
                            &all_people_with_sources.iter()
                                .map(|pws| pws.person.clone())
                                .collect::<Vec<_>>()
                        );
                        println!("✅ Reloaded {} people", all_people_with_sources.len());
                        println!("   Available: {}, Maxed Out: {}", 
                                 stats.available_people, stats.maxed_out_people);
                    }
                    Err(e) => {
                        println!("❌ Error reloading: {}", e);
                    }
                }
            }
            "6" => {
                println!("\n📋 Current Configuration:");
                let config = get_config();
                println!("   • Registry types: {}", config.registry_types.join(", "));
                println!("   • Max selections: {}", config.max_selections);
                println!("   • Max people per file: {}", config.max_people_per_file);
                println!("   • Roster length: {}", config.roster_length);
                println!("   • Folder date format: {}", config.folder_date_format);
                println!("   • File date format: {}", config.file_date_format);
                println!("   • Time format: {}", config.time_format);
                println!("   • File template: {}", config.file_naming_template);
                
                println!("\n📊 Current Registry Stats:");
                let stats = calculate_stats(
                    &all_people_with_sources.iter()
                        .map(|pws| pws.person.clone())
                        .collect::<Vec<_>>()
                );
                println!("   • Total people: {}", stats.total_people);
                println!("   • Available: {} (under {} selections)", 
                         stats.available_people, config.max_selections);
                println!("   • Maxed out: {} ({} selections)", 
                         stats.maxed_out_people, config.max_selections);
            }
            "7" => {
                println!("👋 Goodbye!");
                break;
            }
            _ => println!("❌ Invalid option. Please enter 1-7."),
        }
    }
    
    Ok(())
}

fn display_people(people_with_sources: &[PersonWithSource], available_only: bool) {
    let filtered_people: Vec<&PersonWithSource> = if available_only {
        people_with_sources.iter()
            .filter(|pws| pws.person.is_available())
            .collect()
    } else {
        people_with_sources.iter().collect()
    };
    
    if filtered_people.is_empty() {
        if available_only {
            println!("❌ NO AVAILABLE PEOPLE!");
            println!("   All people have reached maximum selections ({} times).", max_selections());
        } else {
            println!("❌ No people to display.");
        }
        return;
    }
    
    let title = if available_only { "AVAILABLE PEOPLE" } else { "ALL PEOPLE" };
    println!("\n=== {} ===", title);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    for person_with_source in &filtered_people {
        let person = &person_with_source.person;
        let max_sel = max_selections();
        let status = if person.times_selected >= max_sel {
            "❌ MAXED OUT"
        } else {
            "✅ Available"
        };
        
        let source_info = if let Some(date_folder) = &person_with_source.date_folder {
            format!(" ({})", date_folder)
        } else {
            String::new()
        };
        
        println!("• {} - {} ({}){}", 
                 person.name, person.roster, person.birth_date, source_info);
        println!("  Selected: {}/{} times - {}", 
                 person.times_selected, max_sel, status);
        
        if let Some(filename) = person_with_source.file_path.file_name() {
            println!("  File: {}", filename.to_string_lossy());
        }
        println!();
    }
    
    if !available_only {
        let stats = calculate_stats(
            &people_with_sources.iter()
                .map(|pws| pws.person.clone())
                .collect::<Vec<_>>()
        );
        println!("📊 Summary: {} available, {} maxed out (out of {})", 
                 stats.available_people, stats.maxed_out_people, stats.total_people);
    }
    
    println!("Total displayed: {}", filtered_people.len());
}

fn group_people_by_file(people_with_sources: &[PersonWithSource]) -> HashMap<PathBuf, Vec<Person>> {
    let mut file_groups: HashMap<PathBuf, Vec<Person>> = HashMap::new();
    
    for person_with_source in people_with_sources {
        file_groups
            .entry(person_with_source.file_path.clone())
            .or_default()
            .push(person_with_source.person.clone());
    }
    
    file_groups
}

fn update_file_with_people(filepath: &PathBuf, updated_people: &[Person]) -> Result<(), Box<dyn std::error::Error>> {
    if !filepath.exists() {
        return Err(format!("File not found: {:?}", filepath).into());
    }
    
    // Load existing content
    let content = std::fs::read_to_string(filepath)?;
    let lines: Vec<String> = content.lines().map(String::from).collect();
    
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
        return Err("No valid roster table found in file".into());
    }
    
    // Find the table end
    let mut table_end = table_start;
    for i in table_start..lines.len() {
        let trimmed = lines[i].trim();
        if trimmed.is_empty() || !trimmed.starts_with('|') {
            table_end = i;
            break;
        }
    }
    if table_end == table_start {
        table_end = lines.len();
    }
    
    // Create a map of updated people by roster
    let updated_map: std::collections::HashMap<String, &Person> = updated_people
        .iter()
        .map(|p| (p.roster.clone(), p))
        .collect();
    
    // Reconstruct the file with updated data
    let mut new_content = Vec::new();
    
    // Copy everything before the table
    for i in 0..table_start {
        new_content.push(lines[i].clone());
    }
    
    // Update table rows
    for i in table_start..table_end {
        let line = &lines[i];
        let trimmed = line.trim();
        
        if trimmed.is_empty() || !trimmed.starts_with('|') {
            new_content.push(line.clone());
            continue;
        }
        
        let columns: Vec<&str> = trimmed.split('|')
            .skip(1)
            .filter(|s| !s.trim().is_empty())
            .map(|s| s.trim())
            .collect();
        
        if columns.len() >= 4 {
            let roster = columns[1].trim_matches('*').trim();
            
            if let Some(updated_person) = updated_map.get(roster) {
                // Update this row
                let max_selections = max_selections();
                let times_selected_display = if updated_person.times_selected >= max_selections {
                    format!("**{}** (MAX)", updated_person.times_selected)
                } else {
                    format!("{}", updated_person.times_selected)
                };
                
                new_content.push(format!(
                    "| {} | **{}** | {} | {} |",
                    updated_person.name,
                    updated_person.roster,
                    updated_person.birth_date,
                    times_selected_display
                ));
            } else {
                // Keep original row
                new_content.push(line.clone());
            }
        } else {
            new_content.push(line.clone());
        }
    }
    
    // Copy everything after the table
    for i in table_end..lines.len() {
        new_content.push(lines[i].clone());
    }
    
    // Update "Updated" timestamp
    let now = chrono::Local::now();
    let updated_timestamp = format!("*Updated: {}*", now.format("%B %d, %Y at %H:%M:%S"));
    
    for i in 0..new_content.len() {
        if new_content[i].starts_with("*Updated:") {
            new_content[i] = updated_timestamp.clone();
            break;
        }
    }
    
    // If no "Updated" line exists, add it after the "Generated" line
    let mut has_updated = false;
    for line in &new_content {
        if line.starts_with("*Updated:") {
            has_updated = true;
            break;
        }
    }
    
    if !has_updated {
        for i in 0..new_content.len() {
            if new_content[i].starts_with("*Generated:") {
                new_content.insert(i + 1, updated_timestamp);
                break;
            }
        }
    }
    
    // Write the updated file
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(filepath)?;
    
    for line in new_content {
        writeln!(file, "{}", line)?;
    }
    
    Ok(())
}