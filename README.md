# SDR Sortition Service

## Copyright Notice
Copyright © 2024 Geaucef Stone. All rights reserved under the terms of the GNU GPL v3.

## License

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
GNU General Public License for more details.

You should have received a copy of the GNU General Public License
along with this program. If not, see <https://www.gnu.org/licenses/gpl-3.0.html>.

## Plugin/Extension Exception

### GNU GPL v3 Linking Exception for Future Plugins

**Section 7 Exception for Future Plugin System**: Notwithstanding any other provision of the GNU General Public License, 
you have permission to link this program with independent modules ("Plugins") that communicate with the 
program solely through a future plugin application programming interface (API), regardless of the license terms of these 
independent modules, provided that:

1. The independent modules are not derivative works of this program's core functionality
2. The independent modules do not incorporate any portion of this program's source code
3. The independent modules communicate with this program only through a documented, 
   versioned plugin API (to be implemented in future releases)
4. The independent modules are clearly distinguished from the core program
5. The independent modules do not circumvent the normal execution flow or licensing of this program

**Current Status**: This exception is established for future compatibility. No plugin API currently exists in this version.

## How to Apply This License to Your Work
To apply this license to your modifications or distributions:

1. Preserve this copyright notice and license text
2. State any significant changes made to the original
3. Keep all notices that refer to this License and to the absence of any warranty
4. Provide recipients with a copy of the GNU GPL v3
5. Include this plugin exception clause if distributing modified versions

## No Warranty
**THIS SOFTWARE IS PROVIDED "AS IS" WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED,
INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR
PURPOSE, AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE
LIABLE FOR ANY CLAIM, DAMAGES, OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE
OR OTHER DEALINGS IN THE SOFTWARE.**

## Protection Against Monopolization

### Legal Protection (GPL v3 + Future Plugin Exception)
The GNU General Public License v3 with future plugin exception provides balanced legal safeguards:

**Core Protection:**
- **No proprietary forks**: Core program modifications must be shared
- **No exclusive ownership claims**: Core program remains community-owned
- **Permanent freedom**: Core source code remains freely available forever
- **No vendor lock-in**: No single organization can control core access

**Future Plugin Flexibility:**
- **Encourages future innovation**: Developers can plan for proprietary plugins
- **Supports future integration**: Allows connection with other systems
- **Promotes ecosystem growth**: Enables commercial extensions without compromising core freedoms

## Project Overview
These programs are designed for selecting participants in sortition processes
for Citizens' and Workers' Branches in democratic systems.

The system consists of two complementary applications:
1. **Roster Generator**: Creates unique roster codes from registration data
2. **Roster Selector**: Performs weighted random selection from registered rosters

## ⚠️ Current Implementation Status
**Version 0.1.5 - PROTOTYPE**

### What EXISTS Now:
- ✅ Core roster generation and management
- ✅ File-based storage in Markdown format  
- ✅ Weighted random selection algorithms
- ✅ Date-based file organization
- ✅ Duplicate prevention across files
- ✅ Configuration system

### What is PLANNED (Future Versions):
- 🔄 **Plugin System**: Extensible architecture through plugins
- 🔄 **Plugin API**: Documented interfaces for third-party extensions
- 🔄 **Plugin Marketplace**: Ecosystem of community plugins
- 🔄 **Web Interface**: Browser-based administration
- 🔄 **Database Backend**: Optional database storage
- 🔄 **Enhanced Security**: Encryption and access controls

**Note**: The plugin exception clause is included for future compatibility but no plugin system currently exists.

## Security Advisory
**IMPORTANT**: These programs are functional prototypes, not production-grade systems.

### Critical Warnings:
1. **Prototype Status**: This software has not undergone formal security audit
2. **Personal Data**: Do not collect sensitive identifiers (driver's license numbers, 
   national IDs, etc.) without proper legal authority and data protection measures
3. **Legal Compliance**: Users are responsible for complying with applicable laws
   regarding data protection, privacy, and electoral processes in their jurisdiction
4. **Professional Review**: Consult with security and legal professionals before 
   deployment in critical or governmental contexts

## Technical Implementation

### Current Architecture (v0.1.0)
- **Language**: Rust (prototype phase)
- **Storage**: Local Markdown files with roster data
- **Registry Types**: Citizens and Workers registries
- **File Organization**: Date-based folder structure with automatic organization
- **Roster Codes**: 8-character unique identifiers derived from birth dates with collision prevention
- **Selection Limits**: Maximum 4 selections per person, weighted random algorithm
- **Plugin System**: **Not yet implemented** (planned for future versions)

### Two-Application System

#### 1. Roster Generator
- **Purpose**: Creates and manages roster entries
- **Key Features**:
  * Generates unique 8-character roster codes from birth dates
  * Organizes files by date folders (e.g., `citizens-2024-12-20`)
  * Limits to 10 people per file (auto-splits when full)
  * Prevents duplicate roster codes across all files
  * Tracks unique people across entire registry

#### 2. Roster Selector
- **Purpose**: Selects participants from existing rosters
- **Key Features**:
  * Scans ALL files across ALL date folders
  * Weighted random selection (less-selected people have higher chance)
  * Enforces 4-selection maximum per person
  * Updates selection counts in original files
  * Shows available vs. maxed-out statistics

### File Structure
```bash
~/Documents/md-data/
├── citizens/                          # Citizens registry
│   ├── citizens_001_2024_12_27_143022.md
│   ├── citizens_002_2024_12_28_093045.md
│   └── citizens-2024-12-20/          # Date-based organization
│       ├── citizens_001_2024_12_20_121000.md
│       └── citizens_002_2024_12_20_121500.md
└── workers/                          # Workers registry
    ├── workers_001_2024_12_27_152118.md
    └── workers-2024-12-20/
        └── workers_001_2024_12_20_131000.md
```

## Installation and Usage

### Prerequisites

- Rust toolchain (install from rustup.rs)
- Git or SVN (to clone the repository)

### Step 1: Download and Build

```bash
# Clone the repository
git clone https://codeberg.org/GeaucefStone/Sortition.git
cd Sortition

# Build all three components at once (this is a Rust workspace)
cargo build --release
```

The workspace contains:

- roster-core - Shared library (internal)
- roster-gen - Roster generator application
- roster-select - Roster selector application

### Step 2: Run the Applications

```bash 
# Run the roster generator
./target/release/roster-gen
```

### Step 3: Understanding the Flow

1. **First run roster-gen** to create your roster files
- Choose registry type (citizens or workers)
- Enter names and birth dates (MM/DD/YYYY)
- Files are automatically saved in ~/Documents/md-data/

2. **Then run roster-select** to make selections
- Choose the same registry type
- View all available people
- Make random selections
- Selection counts are automatically updated

### Step 4: Configure the System

**Configuration File Location**

When you first run either application, a configuration file is automatically created at:

- Linux/macOS: ~/.config/roster/sortition.ron
- Windows: %APPDATA%\roster\sortition.ron
- Fallback: Current directory: ./.roster/sortition.ron

**Viewing Current Configuration**

Run either application and choose "Show configuration" from the menu to see your current settings.

**Editing the Configuration File**

The configuration file uses RON (Rusty Object Notation) format. Here's the default configuration:

```rust
RosterConfig(
    registry_types: ["citizens", "workers"],
    max_selections: 4,
    max_people_per_file: 10,
    roster_length: 8,
    folder_date_format: "%Y-%m-%d",
    file_date_format: "%Y_%m_%d",
    time_format: "%H%M%S",
    file_naming_template: "{registry}_{seq:03}_{datetime}.md",
)
```

**Configuration Options Explained**

1. **registry_types** - List of registry types you can create

2. **max_selections** - Maximum times a person can be selected

3. **max_people_per_file** - Auto-split files when they reach this size

4. **roster_length** - Length of roster codes (1-20 characters)

