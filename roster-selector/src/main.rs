use rand::seq::SliceRandom;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use chrono::prelude::*;
use dirs;
use std::hash::{Hash, Hasher};

// Define a struct to hold student data (MUST match generator's Person struct)
#[derive(Clone, Debug)]
struct Student {
    name: String,
    roster: String,
    birth_date: String,
    times_selected: u32,
}

// Implement Eq and Hash EXACTLY like generator's Person
impl PartialEq for Student {
    fn eq(&self, other: &Self) -> bool {
        self.roster == other.roster
    }
}

impl Eq for Student {}

impl Hash for Student {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.roster.hash(state);
    }
}

// Track which file a student comes from
#[derive(Debug)]
struct StudentWithSource {
    student: Student,
    file_path: PathBuf,
    date_folder: Option<String>,
}

// Get the base directory matching Roster Generator EXACTLY
fn get_base_directory() -> Result<PathBuf, Box<dyn Error>> {
    // Method 1: Use dirs crate (EXACTLY like Roster Generator)
    if let Some(docs) = dirs::document_dir() {
        let base_dir = docs.join("md-data");
        return Ok(base_dir);
    }
    
    // Method 2: Try environment variables as fallback (same as Roster Generator)
    #[cfg(target_os = "windows")]
    {
        if let Ok(userprofile) = std::env::var("USERPROFILE") {
            let docs = PathBuf::from(userprofile).join("Documents").join("md-data");
            if docs.exists() {
                return Ok(docs);
            }
        }
    }
    
    #[cfg(target_os = "macos")]
    {
        if let Ok(home) = std::env::var("HOME") {
            let docs = PathBuf::from(home).join("Documents").join("md-data");
            if docs.exists() {
                return Ok(docs);
            }
        }
    }
    
    #[cfg(target_os = "linux")]
    {
        if let Ok(home) = std::env::var("HOME") {
            let docs = PathBuf::from(home).join("Documents").join("md-data");
            if docs.exists() {
                return Ok(docs);
            }
        }
    }
    
    // Method 3: Fallback to current directory
    let current_dir = std::env::current_dir()?;
    let base_dir = current_dir.join("md-data");
    Ok(base_dir)
}

// Load ALL students from ALL files and ALL date folders in a registry
fn load_all_students_with_sources(registry_type: &str) -> Result<(Vec<StudentWithSource>, usize, usize), Box<dyn Error>> {
    let base_dir = get_base_directory()?;
    let registry_dir = base_dir.join(registry_type);
    
    if !registry_dir.exists() {
        return Err(format!("Registry directory not found: {:?}", registry_dir).into());
    }
    
    let mut all_students = Vec::new();
    let mut file_count = 0;
    let mut date_folder_count = 0;
    
    println!("📂 Loading ALL data from {} registry...", registry_type);
    
    // Helper to load students from a directory
    fn load_from_directory(dir: &Path, date_folder: Option<String>) -> Result<(Vec<StudentWithSource>, usize), Box<dyn Error>> {
        let mut students_from_dir = Vec::new();
        let mut files_in_dir = 0;
        
        if !dir.exists() {
            return Ok((students_from_dir, files_in_dir));
        }
        
        // List markdown files in this directory
        let mut files = Vec::new();
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == "md" || ext == "markdown" {
                        if let Some(filename) = path.file_name() {
                            if let Some(filename_str) = filename.to_str() {
                                files.push((filename_str.to_string(), path));
                            }
                        }
                    }
                }
            }
        }
        
        // Sort by sequence number then by timestamp (EXACTLY like first app)
        files.sort_by(|(a_name, _), (b_name, _)| {
            let a_parts: Vec<&str> = a_name.split('_').collect();
            let b_parts: Vec<&str> = b_name.split('_').collect();
            
            if a_parts.len() >= 2 && b_parts.len() >= 2 {
                let a_num = a_parts[1].parse::<u32>().unwrap_or(0);
                let b_num = b_parts[1].parse::<u32>().unwrap_or(0);
                a_num.cmp(&b_num)
            } else {
                a_name.cmp(b_name)
            }
        });
        
        // Load each file
        for (filename, filepath) in &files {
            match load_students_from_markdown(filepath) {
                Ok(students) => {
                    files_in_dir += 1;
                    for student in students {
                        students_from_dir.push(StudentWithSource {
                            student,
                            file_path: filepath.clone(),
                            date_folder: date_folder.clone(),
                        });
                    }
                }
                Err(e) => {
                    println!("   ⚠️  {}: Error - {}", filename, e);
                }
            }
        }
        
        Ok((students_from_dir, files_in_dir))
    }
    
    // Load from root registry directory (no date folder)
    let (mut root_students, root_files) = load_from_directory(&registry_dir, None)?;
    all_students.extend(root_students);
    file_count += root_files;
    
    // Load from all date folders
    if registry_dir.exists() {
        for entry in fs::read_dir(&registry_dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_dir() {
                if let Some(dir_name) = path.file_name() {
                    if let Some(dir_name_str) = dir_name.to_str() {
                        // Check if it matches the pattern: registry_type-YYYY-MM-DD
                        if dir_name_str.starts_with(&format!("{}-", registry_type)) {
                            date_folder_count += 1;
                            
                            let (mut folder_students, folder_files) = load_from_directory(&path, Some(dir_name_str.to_string()))?;
                            all_students.extend(folder_students);
                            file_count += folder_files;
                        }
                    }
                }
            }
        }
    }
    
    // Remove duplicates by roster ID using HashSet (like generator does)
    let before = all_students.len();
    let mut unique_students = Vec::new();
    let mut seen_rosters = HashSet::new();
    
    for student_with_source in all_students {
        if !seen_rosters.contains(&student_with_source.student.roster) {
            seen_rosters.insert(student_with_source.student.roster.clone());
            unique_students.push(student_with_source);
        }
    }
    
    let removed = before - unique_students.len();
    if removed > 0 {
        println!("⚠️  Removed {} duplicate roster(s)", removed);
    }
    
    println!("📊 Loaded {} unique students from {} files ({} date folders)", 
             unique_students.len(), file_count, date_folder_count);
    
    Ok((unique_students, file_count, date_folder_count))
}

// Load students from a single markdown file (EXACT parsing as generator)
fn load_students_from_markdown(filepath: &Path) -> Result<Vec<Student>, Box<dyn Error>> {
    if !filepath.exists() {
        return Err(format!("File not found: {:?}", filepath).into());
    }
    
    let content = fs::read_to_string(filepath)?;
    let lines: Vec<&str> = content.lines().collect();
    
    // Find the table start (EXACTLY like generator)
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
        return Err("No valid roster table found".into());
    }
    
    // Parse table rows (EXACTLY like generator)
    let mut students = Vec::new();
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
            
            students.push(Student {
                name,
                roster,
                birth_date,
                times_selected,
            });
        }
    }
    
    Ok(students)
}

// Update a specific file with new student data
fn update_student_file(filepath: &Path, updated_students: &[Student]) -> Result<(), Box<dyn Error>> {
    if !filepath.exists() {
        return Err(format!("File not found: {:?}", filepath).into());
    }
    
    // Read the entire file
    let content = fs::read_to_string(filepath)?;
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
    
    // Create a map of updated students by roster
    let updated_map: HashMap<String, &Student> = updated_students
        .iter()
        .map(|s| (s.roster.clone(), s))
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
            
            if let Some(updated_student) = updated_map.get(roster) {
                // Update this row
                new_content.push(format!(
                    "| {} | **{}** | {} | {} |",
                    updated_student.name,
                    updated_student.roster,
                    updated_student.birth_date,
                    updated_student.times_selected
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
    
    // Update "Updated" timestamp in header
    let now = Local::now();
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
    let mut file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(filepath)?;
    
    for line in new_content {
        writeln!(file, "{}", line)?;
    }
    
    Ok(())
}

fn count_student_status(students: &[StudentWithSource]) -> (usize, usize) {
    let available = students.iter()
        .filter(|s| s.student.times_selected < 4)
        .count();
    let maxed_out = students.len() - available;
    (available, maxed_out)
}

fn display_students(students: &[StudentWithSource], available_only: bool) {
    let filtered_students: Vec<&StudentWithSource> = if available_only {
        students.iter()
            .filter(|s| s.student.times_selected < 4)
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
    
    for student_with_source in &filtered_students {
        let student = &student_with_source.student;
        let status = if student.times_selected >= 4 {
            "❌ MAXED OUT"
        } else {
            "✅ Available"
        };
        
        let source_info = if let Some(date_folder) = &student_with_source.date_folder {
            format!(" ({})", date_folder)
        } else {
            String::new()
        };
        
        println!("• {} - {} ({}){}", 
                 student.name, student.roster, student.birth_date, source_info);
        println!("  Selected: {}/4 times - {}", student.times_selected, status);
        
        if let Some(filename) = student_with_source.file_path.file_name() {
            println!("  File: {}", filename.to_string_lossy());
        }
        println!();
    }
    
    if !available_only {
        let (available, maxed_out) = count_student_status(students);
        println!("📊 Summary: {} available, {} maxed out", available, maxed_out);
    }
    
    println!("Total displayed: {}", filtered_students.len());
}

fn select_students(
    students_with_sources: &mut [StudentWithSource], 
    count: usize
) -> Result<Vec<(Student, PathBuf)>, Box<dyn Error>> {
    const MAX_SELECTIONS: u32 = 4;
    
    // Get indices of available students
    let available_indices: Vec<usize> = students_with_sources.iter()
        .enumerate()
        .filter(|(_, s)| s.student.times_selected < MAX_SELECTIONS)
        .map(|(i, _)| i)
        .collect();
    
    if available_indices.is_empty() {
        println!("❌ No available students to select.");
        return Ok(Vec::new());
    }
    
    let actual_count = count.min(available_indices.len());
    
    if actual_count < count {
        println!("⚠️  Only {} students available, selecting {} instead of {}", 
                 available_indices.len(), actual_count, count);
    }
    
    // Create weighted selection pool (students with fewer selections have more weight)
    let mut weighted_indices = Vec::new();
    for &idx in &available_indices {
        let remaining = MAX_SELECTIONS - students_with_sources[idx].student.times_selected;
        // More weight for students selected fewer times
        for _ in 0..remaining {
            weighted_indices.push(idx);
        }
    }
    
    // If weighted pool is empty, use equal weighting
    let selection_pool = if !weighted_indices.is_empty() {
        weighted_indices
    } else {
        available_indices.clone()
    };
    
    let mut rng = rand::thread_rng();
    let mut selected_indices = Vec::new();
    
    // Try to select unique students
    while selected_indices.len() < actual_count && !selection_pool.is_empty() {
        if let Some(&random_idx) = selection_pool.choose(&mut rng) {
            if !selected_indices.contains(&random_idx) {
                selected_indices.push(random_idx);
            }
        }
    }
    
    // Update selected students and collect results
    let mut selected_results = Vec::new();
    for &idx in &selected_indices {
        students_with_sources[idx].student.times_selected += 1;
        selected_results.push((
            students_with_sources[idx].student.clone(),
            students_with_sources[idx].file_path.clone()
        ));
    }
    
    Ok(selected_results)
}

// Group students by file for efficient updating
fn group_students_by_file(students_with_sources: &[StudentWithSource]) -> HashMap<PathBuf, Vec<Student>> {
    let mut file_groups: HashMap<PathBuf, Vec<Student>> = HashMap::new();
    
    for student_with_source in students_with_sources {
        file_groups
            .entry(student_with_source.file_path.clone())
            .or_default()
            .push(student_with_source.student.clone());
    }
    
    file_groups
}

fn main() -> Result<(), Box<dyn Error>> {
    println!("╔══════════════════════════════════════╗");
    println!("║     Markdown Roster Selector         ║");
    println!("║   (Scans ALL files & date folders)   ║");
    println!("║      (Updates files in-place)        ║");
    println!("╚══════════════════════════════════════╝");
    
    println!("\nChoose registry type:");
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
    
    println!("\n🔧 Initializing {} registry selector...", registry_type);
    
    // Get base directory
    let base_dir = get_base_directory()?;
    let registry_dir = base_dir.join(registry_type);
    
    // Check if directory exists
    if !registry_dir.exists() {
        println!("\n❌ ERROR: Registry directory does not exist: {:?}", registry_dir);
        println!("💡 You need to run the Roster Generator first!");
        println!("   Expected location: {:?}", registry_dir);
        return Ok(());
    }
    
    // Load ALL students from ALL files and date folders
    println!("\n📥 Loading ALL students from {} registry...", registry_type);
    let (mut all_students_with_sources, file_count, date_folder_count) = 
        match load_all_students_with_sources(registry_type) {
            Ok((students, files, folders)) => {
                if students.is_empty() {
                    println!("❌ No students found in registry.");
                    println!("💡 Run the Roster Generator to create roster files first.");
                    return Ok(());
                }
                println!("✅ Successfully loaded {} unique students", students.len());
                println!("   From {} files in {} date folders", files, folders);
                (students, files, folders)
            },
            Err(e) => {
                println!("❌ Error loading students: {}", e);
                return Ok(());
            }
        };
    
    // Show initial status
    let (available, maxed_out) = count_student_status(&all_students_with_sources);
    println!("\n📊 Initial Status:");
    println!("   Available: {} students (selected less than 4 times)", available);
    println!("   Maxed Out: {} students (selected 4 times)", maxed_out);
    println!("   Files scanned: {}, Date folders: {}", file_count, date_folder_count);
    
    // Main selection loop
    loop {
        println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("Options:");
        println!("  1. View all students (all files/folders)");
        println!("  2. View available students only");
        println!("  3. Select random students");
        println!("  4. Reset ALL selection counts to 0");
        println!("  5. Reload data from files");
        println!("  6. Exit");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("Enter your choice (1-6):");
        
        let mut option = String::new();
        io::stdin().read_line(&mut option)?;
        
        match option.trim() {
            "1" => display_students(&all_students_with_sources, false),
            "2" => {
                let (available, _) = count_student_status(&all_students_with_sources);
                if available == 0 {
                    println!("❌ No available students!");
                } else {
                    display_students(&all_students_with_sources, true);
                }
            }
            "3" => {
                let (available, _) = count_student_status(&all_students_with_sources);
                if available == 0 {
                    println!("❌ No available students to select!");
                    println!("   All {} students have reached the maximum of 4 selections.", 
                             all_students_with_sources.len());
                    println!("   Use option 4 to reset counts if needed.");
                    continue;
                }
                
                println!("How many students to select? (1-{})", available);
                let mut count_str = String::new();
                io::stdin().read_line(&mut count_str)?;
                
                match count_str.trim().parse::<usize>() {
                    Ok(count) if count >= 1 && count <= available => {
                        println!("\n🎲 Selecting {} student(s)...", count);
                        
                        match select_students(&mut all_students_with_sources, count) {
                            Ok(selected) => {
                                if selected.is_empty() {
                                    println!("❌ No students were selected.");
                                } else {
                                    println!("\n✅ Selected {} student(s):", selected.len());
                                    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
                                    
                                    for (student, file_path) in &selected {
                                        println!("• {} - {} ({})", 
                                                 student.name, student.roster, student.birth_date);
                                        println!("  Now selected: {}/4 times", student.times_selected);
                                        
                                        if let Some(filename) = file_path.file_name() {
                                            println!("  File: {}", filename.to_string_lossy());
                                        }
                                        
                                        if student.times_selected >= 4 {
                                            println!("  ⚠️  MAXED OUT - Will not be selected again");
                                        }
                                        println!();
                                    }
                                    
                                    // Update the source files
                                    println!("💾 Updating source files...");
                                    let file_groups = group_students_by_file(&all_students_with_sources);
                                    let mut files_updated = 0;
                                    
                                    for (filepath, students) in file_groups {
                                        match update_student_file(&filepath, &students) {
                                            Ok(_) => {
                                                files_updated += 1;
                                            }
                                            Err(e) => {
                                                println!("⚠️  Error updating {:?}: {}", 
                                                         filepath.file_name().unwrap_or_default(), e);
                                            }
                                        }
                                    }
                                    
                                    // Update status
                                    let (new_available, new_maxed) = count_student_status(&all_students_with_sources);
                                    println!("\n📊 Updated Status:");
                                    println!("   Available: {} students", new_available);
                                    println!("   Maxed Out: {} students", new_maxed);
                                    println!("   Files updated: {}", files_updated);
                                }
                            }
                            Err(e) => {
                                println!("❌ Error during selection: {}", e);
                            }
                        }
                    }
                    Ok(_) => println!("❌ Please enter a number between 1 and {}", available),
                    Err(_) => println!("❌ Invalid number."),
                }
            }
            "4" => {
                println!("⚠️  Are you sure you want to reset ALL selection counts to 0? (yes/no)");
                println!("   This will update ALL files in ALL date folders.");
                
                let mut confirm = String::new();
                io::stdin().read_line(&mut confirm)?;
                
                if confirm.trim().eq_ignore_ascii_case("yes") {
                    println!("🔄 Resetting ALL selection counts...");
                    
                    // Reset in memory
                    for student_with_source in &mut all_students_with_sources {
                        student_with_source.student.times_selected = 0;
                    }
                    
                    // Group by file and update each file
                    let file_groups = group_students_by_file(&all_students_with_sources);
                    let mut files_updated = 0;
                    
                    for (filepath, students) in file_groups {
                        match update_student_file(&filepath, &students) {
                            Ok(_) => {
                                files_updated += 1;
                            }
                            Err(e) => {
                                println!("⚠️  Error updating {:?}: {}", 
                                         filepath.file_name().unwrap_or_default(), e);
                            }
                        }
                    }
                    
                    println!("✅ Reset complete!");
                    println!("   Updated {} file(s)", files_updated);
                    println!("   All {} students now have 0 selections", all_students_with_sources.len());
                } else {
                    println!("Reset cancelled.");
                }
            }
            "5" => {
                println!("🔄 Reloading data from files...");
                match load_all_students_with_sources(registry_type) {
                    Ok((students, files, folders)) => {
                        all_students_with_sources = students;
                        let (available, maxed_out) = count_student_status(&all_students_with_sources);
                        println!("✅ Reloaded {} students", all_students_with_sources.len());
                        println!("   From {} files in {} date folders", files, folders);
                        println!("   Available: {}, Maxed Out: {}", available, maxed_out);
                    }
                    Err(e) => {
                        println!("❌ Error reloading: {}", e);
                    }
                }
            }
            "6" => {
                println!("👋 Goodbye!");
                break;
            }
            _ => println!("❌ Invalid option. Please enter 1-6."),
        }
    }
    
    Ok(())
}