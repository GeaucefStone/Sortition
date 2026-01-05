use rand::Rng;
use rand::seq::SliceRandom;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

fn main() {
    println!("CSV Roster Selector");
    println!("Enter the path to your CSV file:");

    let mut filename = String::new();
    std::io::stdin().read_line(&mut filename).unwrap();
    let filename = filename.trim();

    let mut students = read_students_from_csv(filename);

    if students.is_empty() {
        println!("No students found in the file or file doesn't exist.");
        println!("Please create a CSV file with this format:");
        println!("Name,Roster,Birth Date,Times Selected");
        println!("Example: John Smith,AAAAAAAA,01/15/1990,0");
        return;
    }

    println!("Successfully loaded {} students from CSV", students.len());
    
    // Filter out maxed out students for display
    let available_students: Vec<&Student> = students.iter()
        .filter(|s| s.times_selected < 4)
        .collect();

    println!("\n=== AVAILABLE STUDENTS ===");
    for (i, student) in available_students.iter().enumerate() {
        println!("  {}. {} - {} - Born: {} (Selected {} times total)", 
                 i + 1, student.name, student.roster, student.birth_date, 
                 student.times_selected);
    }

    let maxed_out_count = students.len() - available_students.len();
    println!("\n{} students available for selection", available_students.len());
    println!("{} students have reached maximum (4) selections and are hidden", maxed_out_count);
    println!("Enter how many people to select (1-1000), or 'q' to quit:");

    loop {
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).unwrap();
        
        let input = input.trim();
        
        if input.eq_ignore_ascii_case("q") {
            break;
        }

        // Parse the number of people to select
        let count: usize = match input.parse() {
            Ok(n) if n > 0 && n <= 1000 => n,
            Ok(n) if n > 1000 => {
                println!("Maximum selection count is 1000. Please enter a smaller number.");
                continue;
            }
            Ok(_) => {
                println!("Please enter a positive number.");
                continue;
            }
            Err(_) => {
                println!("Please enter a valid number (1-1000) or 'q' to quit.");
                continue;
            }
        };

        // Select the specified number of random people
        let selected_indices = select_people(&mut students, count);
        
        if !selected_indices.is_empty() {
            println!("\n=== SELECTED FOR THIS SESSION ===");
            
            // Update counts and display results for the selected people
            for &index in &selected_indices {
                let student = &mut students[index];
                
                // Update the LIFETIME selection count
                student.times_selected += 1;
                
                println!("  {} - {} - Born: {}", 
                         student.name, student.roster, student.birth_date);
                println!("    Now selected {}/4 times LIFETIME", student.times_selected);
                
                if student.times_selected >= 4 {
                    println!("    *** MAXED OUT - Will never be selected again ***");
                }
                println!();
            }
            
            // Save the updated LIFETIME counts to the CSV
            if let Err(e) = save_selection_counts_to_csv(filename, &students) {
                println!("Error saving selection counts: {}", e);
            }
            
            // Display updated counts (only show available students)
            let available_students: Vec<&Student> = students.iter()
                .filter(|s| s.times_selected < 4)
                .collect();
            let maxed_out_count = students.len() - available_students.len();
            
            println!("Updated counts: {} available, {} maxed out (hidden)", 
                     available_students.len(), maxed_out_count);
            
        } else {
            println!("No available students to select.");
            println!("All {} students have reached maximum (4) selections.", students.len());
        }

        println!("\nEnter how many people to select (1-1000), or 'q' to quit:");
    }
}

// Define a struct to hold student data
#[derive(Clone)]
struct Student {
    name: String,
    roster: String,
    birth_date: String,
    times_selected: u32,  // LIFETIME count across all sessions
}

fn select_people(students: &mut [Student], count: usize) -> Vec<usize> {
    let max_selections = 4;
    
    // Get all available students (those who haven't reached lifetime max)
    let available_students: Vec<(usize, &Student)> = students
        .iter()
        .enumerate()
        .filter(|(_, student)| student.times_selected < max_selections)
        .collect();
    
    // If we don't have enough available students, return what we can get
    if available_students.is_empty() {
        return Vec::new();
    }
    
    let available_count = available_students.len();
    let actual_count = count.min(available_count);
    
    if actual_count < count {
        println!("Warning: Only {} students available, selecting {} instead of {}", 
                 available_count, actual_count, count);
    }
    
    // Create weighted selection pool based on remaining lifetime selections
    let mut weighted_indices = Vec::new();
    
    for (index, student) in &available_students {
        let remaining = max_selections - student.times_selected;
        
        // Add the student index multiple times based on remaining LIFETIME selections
        for _ in 0..remaining {
            weighted_indices.push(*index);
        }
    }
    
    // Shuffle the weighted pool and pick the specified number of unique students
    let mut rng = rand::thread_rng();
    let mut selected_indices = Vec::new();
    let mut attempts = 0;
    let max_attempts = 1000; // Prevent infinite loop
    
    while selected_indices.len() < actual_count && attempts < max_attempts {
        if let Some(&random_index) = weighted_indices.choose(&mut rng) {
            if !selected_indices.contains(&random_index) {
                selected_indices.push(random_index);
            }
        }
        attempts += 1;
    }
    
    // If we couldn't get enough unique students with weights, just pick randomly from available
    if selected_indices.len() < actual_count {
        selected_indices.clear();
        let mut available_indices: Vec<usize> = available_students.into_iter().map(|(idx, _)| idx).collect();
        available_indices.shuffle(&mut rng);
        selected_indices = available_indices.into_iter().take(actual_count).collect();
    }
    
    selected_indices
}

fn read_students_from_csv(filename: &str) -> Vec<Student> {
    let mut students = Vec::new();

    let path = Path::new(filename);
    if !path.exists() {
        println!("File '{}' does not exist.", filename);
        return students;
    }

    match File::open(filename) {
        Ok(file) => {
            let reader = BufReader::new(file);
            for (line_number, line) in reader.lines().enumerate() {
                match line {
                    Ok(line) => {
                        // Skip empty lines
                        if line.trim().is_empty() {
                            continue;
                        }
                        
                        let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
                        
                        // Skip header line
                        if line_number == 0 && (parts[0].eq_ignore_ascii_case("name") || parts.contains(&"Name")) {
                            continue;
                        }
                        
                        if parts.len() >= 3 {
                            let name = parts[0].to_string();
                            let roster = parts[1].to_string();
                            let birth_date = parts[2].to_string();
                            
                            // Read LIFETIME selection count (default to 0 if not present or invalid)
                            let times_selected = if parts.len() >= 4 {
                                parts[3].parse().unwrap_or(0)
                            } else {
                                0
                            };
                            
                            if !name.is_empty() {
                                students.push(Student {
                                    name,
                                    roster,
                                    birth_date,
                                    times_selected,
                                });
                            }
                        }
                    }
                    Err(e) => {
                        println!("Error reading line {}: {}", line_number + 1, e);
                    }
                }
            }
            println!("Successfully read {} students from CSV", students.len());
        }
        Err(e) => {
            println!("Error opening CSV file: {}", e);
        }
    }

    students
}

fn save_selection_counts_to_csv(filename: &str, students: &[Student]) -> Result<(), std::io::Error> {
    let mut file = File::create(filename)?;
    
    // Write header
    writeln!(file, "Name,Roster,Birth Date,Times Selected")?;
    
    // Write student data with updated LIFETIME counts
    for student in students {
        writeln!(file, "{},{},{},{}", student.name, student.roster, student.birth_date, student.times_selected)?;
    }
    
    // Flush to ensure data is written
    file.flush()?;
    
    Ok(())
}