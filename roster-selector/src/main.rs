use rand::seq::SliceRandom;
use std::error::Error;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use chrono::prelude::*;

// Define a struct to hold student data
#[derive(Clone, Debug)]
struct Student {
    name: String,
    roster: String,
    birth_date: String,
    times_selected: u32,
}

// Get the base directory matching Roster Generator
fn get_base_directory() -> Result<PathBuf, Box<dyn Error>> {
    // Method 1: Use dirs crate (just like Roster Generator)
    if let Some(docs) = dirs::document_dir() {
        let base_dir = docs.join("md-data");
        println!("📁 Using Documents/md-data: {:?}", base_dir);
        return Ok(base_dir);
    }
    
    // Method 2: Try environment variables as fallback
    #[cfg(target_os = "windows")]
    {
        if let Ok(userprofile) = std::env::var("USERPROFILE") {
            let docs = PathBuf::from(userprofile).join("Documents").join("md-data");
            println!("📁 Using USERPROFILE/Documents/md-data: {:?}", docs);
            if docs.exists() {
                return Ok(docs);
            }
        }
    }
    
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        if let Ok(home) = std::env::var("HOME") {
            let docs = PathBuf::from(home).join("Documents").join("md-data");
            println!("📁 Using HOME/Documents/md-data: {:?}", docs);
            if docs.exists() {
                return Ok(docs);
            }
        }
    }
    
    // Method 3: Current directory as last resort
    let current_dir = std::env::current_dir()?;
    let base_dir = current_dir.join("md-data");
    println!("⚠️  Could not find Documents directory, using current directory: {:?}", base_dir);
    Ok(base_dir)
}

// Helper function to load and process a file
fn load_and_process(filepath: &Path, display_name: &str) -> Result<(), Box<dyn Error>> {
    let mut students = match load_students_from_markdown(filepath) {
        Ok(students) if !students.is_empty() => students,
        Ok(_) => {
            println!("❌ No students found in the file.");
            return Ok(());
        }
        Err(e) => {
            println!("❌ Error loading file: {}", e);
            return Err(e);
        }
    };
    
    println!("✅ Successfully loaded {} students", students.len());
    
    // Show current status
    let (available, maxed_out) = count_student_status(&students);
    println!("\n📊 Current Status:");
    println!("   Available: {} students (selected less than 4 times)", available);
    println!("   Maxed Out: {} students (selected 4 times)", maxed_out);
    
    // Main selection loop
    loop {
        println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("Options:");
        println!("  1. View all students");
        println!("  2. View available students only");
        println!("  3. Select random students");
        println!("  4. Reset all selection counts");
        println!("  5. Save and exit");
        println!("  6. Exit without saving");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("Enter your choice (1-6):");
        
        let mut option = String::new();
        io::stdin().read_line(&mut option)?;
        
        match option.trim() {
            "1" => display_students(&students, false),
            "2" => {
                let (available, _) = count_student_status(&students);
                if available == 0 {
                    println!("❌ No available students!");
                } else {
                    display_students(&students, true);
                }
            }
            "3" => {
                let (available, _) = count_student_status(&students);
                if available == 0 {
                    println!("❌ No available students to select!");
                    println!("   All {} students have reached the maximum of 4 selections.", students.len());
                    println!("   Use option 4 to reset counts if needed.");
                    continue;
                }
                
                println!("How many students to select? (1-{})", available);
                let mut count_str = String::new();
                io::stdin().read_line(&mut count_str)?;
                
                match count_str.trim().parse::<usize>() {
                    Ok(count) if count >= 1 && count <= available => {
                        let selected = select_students(&mut students, count);
                        
                        if selected.is_empty() {
                            println!("❌ No students available for selection.");
                        } else {
                            println!("\n✅ Selected {} student(s):", selected.len());
                            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
                            
                            for student in &selected {
                                println!("• {} - {} ({})", 
                                         student.name, student.roster, student.birth_date);
                                println!("  Now selected: {}/4 times", student.times_selected);
                                
                                if student.times_selected >= 4 {
                                    println!("  ⚠️  MAXED OUT - Will not be selected again");
                                }
                                println!();
                            }
                            
                            // Update counts
                            let (new_available, new_maxed) = count_student_status(&students);
                            println!("📊 Updated Status:");
                            println!("   Available: {} students", new_available);
                            println!("   Maxed Out: {} students", new_maxed);
                            
                            // Auto-save after selection
                            if let Err(e) = save_to_markdown(filepath, &students) {
                                println!("❌ Error saving: {}", e);
                            } else {
                                println!("💾 Auto-saved changes to {}", display_name);
                            }
                        }
                    }
                    Ok(_) => println!("❌ Please enter a number between 1 and {}", available),
                    Err(_) => println!("❌ Invalid number."),
                }
            }
            "4" => {
                println!("⚠️  Are you sure you want to reset ALL selection counts to 0? (yes/no)");
                let mut confirm = String::new();
                io::stdin().read_line(&mut confirm)?;
                
                if confirm.trim().eq_ignore_ascii_case("yes") {
                    for student in &mut students {
                        student.times_selected = 0;
                    }
                    println!("✅ All selection counts reset to 0.");
                    
                    if let Err(e) = save_to_markdown(filepath, &students) {
                        println!("❌ Error saving: {}", e);
                    } else {
                        println!("💾 Saved changes to {}", display_name);
                    }
                } else {
                    println!("Reset cancelled.");
                }
            }
            "5" => {
                if let Err(e) = save_to_markdown(filepath, &students) {
                    println!("❌ Error saving: {}", e);
                } else {
                    println!("✅ Changes saved to {}", display_name);
                }
                println!("👋 Goodbye!");
                break;
            }
            "6" => {
                println!("⚠️  Exit without saving? All changes will be lost! (yes/no)");
                let mut confirm = String::new();
                io::stdin().read_line(&mut confirm)?;
                
                if confirm.trim().eq_ignore_ascii_case("yes") {
                    println!("👋 Goodbye!");
                    break;
                }
            }
            _ => println!("❌ Invalid option. Please enter 1-6."),
        }
    }
    
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    println!("╔══════════════════════════════════════╗");
    println!("║     Markdown Roster Selector         ║");
    println!("╚══════════════════════════════════════╝");
    
    println!("\nChoose registry type:");
    println!("1. Citizens registry");
    println!("2. Workers registry");
    println!("3. Show all available files");
    
    let mut choice = String::new();
    io::stdin().read_line(&mut choice)?;
    
    let registry_filter = match choice.trim() {
        "1" => Some("citizens"),
        "2" => Some("workers"),
        "3" => None,
        _ => {
            println!("⚠️  Invalid choice. Showing all files.");
            None
        }
    };
    
    // Get the base directory FIRST
    let base_dir = get_base_directory()?;
    println!("📁 Base directory: {:?}", base_dir);
    
    // Check if directory exists
    if !base_dir.exists() {
        println!("\n❌ ERROR: Directory does not exist: {:?}", base_dir);
        println!("💡 You need to run the Roster Generator first!");
        println!("   The Roster Generator will create: {:?}", base_dir);
        println!("\nWould you like to create it now? (yes/no)");
        
        let mut create_choice = String::new();
        io::stdin().read_line(&mut create_choice)?;
        
        if create_choice.trim().eq_ignore_ascii_case("yes") {
            fs::create_dir_all(&base_dir)?;
            println!("✅ Created directory: {:?}", base_dir);
            println!("   Now run the Roster Generator to create registry files.");
        } else {
            println!("❌ Cannot continue without directory. Exiting.");
        }
        return Ok(());
    }
    
    // List files
    let files = list_markdown_files(&base_dir, registry_filter)?;
    
    if files.is_empty() {
        println!("\n❌ No matching roster files found in: {:?}", base_dir);
        
        if let Some(filter) = registry_filter {
            let registry_dir = base_dir.join(filter);
            println!("   Looking in: {:?}", registry_dir);
            
            if !registry_dir.exists() {
                println!("   ⚠️  Directory doesn't exist!");
                println!("   💡 Run 'Roster Generator' and create a {} registry first.", filter);
            } else {
                println!("   Directory exists but contains no .md files.");
            }
        }
        
        println!("\n💡 Make sure to:");
        println!("   1. Run the 'Roster Generator' application first");
        println!("   2. Create a registry and save it");
        return Ok(());
    }
    
    println!("\n📂 Available roster files:");
    for (i, (display_name, path)) in files.iter().enumerate() {
        println!("   {}. {} ({:?})", i + 1, display_name, path);
    }
    
    println!("\nSelect a file to load (enter number):");
    let mut choice = String::new();
    io::stdin().read_line(&mut choice)?;
    
    let (display_name, filepath) = match choice.trim().parse::<usize>() {
        Ok(choice_num) if choice_num >= 1 && choice_num <= files.len() => {
            files[choice_num - 1].clone()
        }
        _ => {
            println!("❌ Invalid selection. Using first file.");
            files[0].clone()
        }
    };
    
    println!("\n📖 Loading: {}", display_name);
    
    load_and_process(&PathBuf::from(filepath), &display_name)
}

fn list_markdown_files(base_dir: &Path, registry_filter: Option<&str>) -> Result<Vec<(String, String)>, Box<dyn Error>> {
    let mut files = Vec::new();
    
    println!("📂 Searching in: {:?}", base_dir);
    
    // Check specific registry directory if filter provided
    if let Some(filter) = registry_filter {
        let registry_dir = base_dir.join(filter);
        println!("📁 Looking in registry directory: {:?}", registry_dir);
        
        if registry_dir.exists() {
            for entry in fs::read_dir(&registry_dir)? {
                let entry = entry?;
                let path = entry.path();
                
                if path.is_file() {
                    if let Some(extension) = path.extension() {
                        if extension == "md" || extension == "markdown" {
                            if let Some(filename) = path.file_name() {
                                // Store with directory prefix
                                let display_name = format!("{}/{}", filter, filename.to_string_lossy());
                                let full_path = path.to_string_lossy().to_string();
                                files.push((display_name, full_path));
                            }
                        }
                    }
                }
            }
        } else {
            println!("⚠️  Registry directory doesn't exist: {:?}", registry_dir);
        }
    } else {
        // No filter, search all registry directories
        for registry in ["citizens", "workers"].iter() {
            let registry_dir = base_dir.join(registry);
            
            if registry_dir.exists() {
                if let Ok(entries) = fs::read_dir(&registry_dir) {
                    for entry in entries {
                        let entry = entry?;
                        let path = entry.path();
                        
                        if path.is_file() {
                            if let Some(extension) = path.extension() {
                                if extension == "md" || extension == "markdown" {
                                    if let Some(filename) = path.file_name() {
                                        let display_name = format!("{}/{}", registry, filename.to_string_lossy());
                                        let full_path = path.to_string_lossy().to_string();
                                        files.push((display_name, full_path));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // Also check base directory directly for backward compatibility
        for entry in fs::read_dir(base_dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_file() {
                if let Some(extension) = path.extension() {
                    if extension == "md" || extension == "markdown" {
                        if let Some(filename) = path.file_name() {
                            let display_name = filename.to_string_lossy().to_string();
                            let full_path = path.to_string_lossy().to_string();
                            files.push((display_name, full_path));
                        }
                    }
                }
            }
        }
    }
    
    files.sort_by(|a, b| a.0.cmp(&b.0));
    
    if files.is_empty() {
        println!("❌ No markdown files found.");
    } else {
        println!("✅ Found {} file(s)", files.len());
    }
    
    Ok(files)
}

fn load_students_from_markdown(filepath: &Path) -> Result<Vec<Student>, Box<dyn Error>> {
    if !filepath.exists() {
        return Err(format!("File not found: {:?}", filepath).into());
    }
    
    println!("📄 Loading from: {:?}", filepath);
    
    let content = fs::read_to_string(filepath)?;
    let lines: Vec<&str> = content.lines().collect();
    
    // Find table start
    let mut table_start = None;
    for (i, line) in lines.iter().enumerate() {
        if line.contains("| Name | Roster |") || (line.contains("| Name |") && line.contains("Roster")) {
            if let Some(next_line) = lines.get(i + 1) {
                if next_line.contains("---") || next_line.contains("| --") {
                    table_start = Some(i + 2);
                    break;
                }
            }
        }
    }
    
    let table_start = match table_start {
        Some(start) => start,
        None => return Err("No valid roster table found".into()),
    };
    
    // Parse rows
    let mut students = Vec::new();
    for line in &lines[table_start..] {
        let trimmed = line.trim();
        if trimmed.is_empty() || !trimmed.starts_with('|') {
            continue;
        }
        
        let columns: Vec<String> = trimmed.split('|')
            .skip(1)
            .take_while(|s| !s.trim().is_empty() && !s.trim().starts_with("---"))
            .map(|s| s.trim().to_string())
            .collect();
        
        if columns.len() >= 4 {
            let name = columns[0]
                .replace("**", "")
                .replace("*", "")
                .trim()
                .to_string();
            
            let roster = columns[1]
                .replace("**", "")
                .replace("*", "")
                .trim()
                .to_string();
            
            let birth_date = columns[2].trim().to_string();
            
            let times_selected_cleaned = columns[3]
                .replace("**", "")
                .replace("*", "")
                .trim()
                .to_string();
            
            let times_selected = times_selected_cleaned.parse().unwrap_or(0);
            
            students.push(Student {
                name,
                roster,
                birth_date,
                times_selected,
            });
        }
    }
    
    if students.is_empty() {
        return Err("No student data found".into());
    }
    
    println!("✅ Parsed {} students", students.len());
    Ok(students)
}

fn save_to_markdown(filepath: &Path, students: &[Student]) -> Result<(), Box<dyn Error>> {
    println!("💾 Saving to: {:?}", filepath);
    
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(filepath)?;
    
    let now = Local::now();
    
    // Write Markdown header with metadata
    writeln!(file, "# Roster Registry")?;
    writeln!(file)?;
    writeln!(file, "*Last Updated: {}*", now.format("%B %d, %Y at %H:%M:%S"))?;
    writeln!(file, "*Total Entries: {}*", students.len())?;
    
    // Calculate statistics
    let (available, maxed_out) = count_student_status(students);
    writeln!(file, "*Available for Selection: {}*", available)?;
    writeln!(file, "*Maxed Out: {}*", maxed_out)?;
    writeln!(file)?;
    
    // Write table header
    writeln!(file, "## Roster List")?;
    writeln!(file)?;
    writeln!(file, "| Name | Roster | Birth Date | Times Selected |")?;
    writeln!(file, "| :--- | :----- | :--------- | :------------- |")?;
    
    // Write student rows
    for student in students {
        let times_display = if student.times_selected >= 4 {
            format!("**{}**", student.times_selected)
        } else {
            student.times_selected.to_string()
        };
        
        writeln!(
            file, 
            "| {} | **{}** | {} | {} |", 
            student.name, 
            student.roster, 
            student.birth_date, 
            times_display
        )?;
    }
    
    writeln!(file)?;
    
    // Write statistics section
    writeln!(file, "## Selection Statistics")?;
    writeln!(file)?;
    writeln!(file, "- **Total People:** {}", students.len())?;
    writeln!(file, "- **Available for Selection:** {}", available)?;
    writeln!(file, "- **Maxed Out (4 selections):** {}", maxed_out)?;
    
    // Calculate average selections
    if !students.is_empty() {
        let total_selections: u32 = students.iter().map(|s| s.times_selected).sum();
        let average = total_selections as f32 / students.len() as f32;
        writeln!(file, "- **Average Selections per Person:** {:.2}", average)?;
    }
    
    // List most selected students
    let mut sorted_students: Vec<&Student> = students.iter().collect();
    sorted_students.sort_by(|a, b| b.times_selected.cmp(&a.times_selected).then(a.name.cmp(&b.name)));
    
    if !sorted_students.is_empty() && sorted_students[0].times_selected > 0 {
        writeln!(file)?;
        writeln!(file, "### Most Selected")?;
        writeln!(file)?;
        
        let mut count = 0;
        for student in sorted_students {
            if student.times_selected > 0 && count < 5 {
                count += 1;
                writeln!(file, "{}. {} - {} times", count, student.name, student.times_selected)?;
            }
        }
    }
    
    writeln!(file)?;
    writeln!(file, "---")?;
    writeln!(file, "*File: {}*", filepath.file_name().unwrap_or_default().to_string_lossy())?;
    writeln!(file, "*Updated by Roster Selector v0.1.0*")?;
    writeln!(file, "*Location: {:?}*", filepath.parent().unwrap_or_else(|| Path::new(".")))?;
    
    println!("✅ File saved successfully");
    Ok(())
}

fn count_student_status(students: &[Student]) -> (usize, usize) {
    let available = students.iter()
        .filter(|s| s.times_selected < 4)
        .count();
    let maxed_out = students.len() - available;
    (available, maxed_out)
}

fn display_students(students: &[Student], available_only: bool) {
    let filtered_students: Vec<&Student> = if available_only {
        students.iter()
            .filter(|s| s.times_selected < 4)
            .collect()
    } else {
        students.iter().collect()
    };
    
    if filtered_students.is_empty() {
        if available_only {
            println!("❌ NO AVAILABLE STUDENTS!");
            println!("   All students have reached maximum selections (4 times).");
        } else {
            println!("❌ No students to display.");
        }
        return;
    }
    
    let title = if available_only { "AVAILABLE STUDENTS" } else { "ALL STUDENTS" };
    println!("\n=== {} ===", title);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    for student in &filtered_students {
        let status = if student.times_selected >= 4 {
            "❌ MAXED OUT (WILL NOT BE SELECTED AGAIN)"
        } else {
            "✅ Available for selection"
        };
        
        println!("• {} - {} ({})", student.name, student.roster, student.birth_date);
        println!("  Selected: {}/4 times - {}", student.times_selected, status);
        println!();
    }
    
    if !available_only {
        let (available, maxed_out) = count_student_status(students);
        println!("📊 Summary: {} available, {} maxed out", available, maxed_out);
    }
    
    println!("Total displayed: {}", filtered_students.len());
}

fn select_students(students: &mut [Student], count: usize) -> Vec<Student> {
    const MAX_SELECTIONS: u32 = 4;
    
    // Get indices of available students (ONLY those with < 4 selections)
    let available_indices: Vec<usize> = students.iter()
        .enumerate()
        .filter(|(_, s)| s.times_selected < MAX_SELECTIONS)
        .map(|(i, _)| i)
        .collect();
    
    if available_indices.is_empty() || count == 0 {
        println!("⚠️  No available students to select.");
        return Vec::new();
    }
    
    let actual_count = count.min(available_indices.len());
    
    if actual_count < count {
        println!("⚠️  Only {} students available, selecting {} instead of {}", 
                 available_indices.len(), actual_count, count);
    }
    
    // Create weighted selection pool
    let mut weighted_indices = Vec::new();
    for &idx in &available_indices {
        let remaining = MAX_SELECTIONS - students[idx].times_selected;
        // More weight for students selected fewer times
        for _ in 0..remaining {
            weighted_indices.push(idx);
        }
    }
    
    // If weighted pool is empty (all at 3 selections), use equal weighting
    if weighted_indices.is_empty() {
        weighted_indices = available_indices.clone();
    }
    
    let mut rng = rand::thread_rng();
    let mut selected_indices = Vec::new();
    let mut attempts = 0;
    let max_attempts = 1000;
    
    // Try to select unique students with weighting
    while selected_indices.len() < actual_count && attempts < max_attempts {
        if let Some(&random_idx) = weighted_indices.choose(&mut rng) {
            if !selected_indices.contains(&random_idx) {
                selected_indices.push(random_idx);
            }
        }
        attempts += 1;
    }
    
    // Fallback: if we can't get enough unique with weights
    if selected_indices.len() < actual_count {
        println!("⚠️  Couldn't get unique selection with weights, using random selection.");
        selected_indices.clear();
        let mut shuffled: Vec<usize> = available_indices;
        shuffled.shuffle(&mut rng);
        selected_indices = shuffled.into_iter().take(actual_count).collect();
    }
    
    // SAFETY CHECK - Double verify no one is at 4 selections
    for &idx in &selected_indices {
        if students[idx].times_selected >= MAX_SELECTIONS {
            println!("❌ ERROR: Student {} already has {} selections! Skipping.", 
                     students[idx].name, students[idx].times_selected);
            return Vec::new();
        }
    }
    
    // Update selected students and collect results
    let mut selected_students = Vec::new();
    for &idx in &selected_indices {
        students[idx].times_selected += 1;
        
        selected_students.push(students[idx].clone());
    }
    
    selected_students
}