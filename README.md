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

## How to Apply This License to Your Work
To apply this license to your modifications or distributions:
1. Preserve this copyright notice and license text
2. State any significant changes made to the original
3. Keep all notices that refer to this License and to the absence of any warranty
4. Provide recipients with a copy of the GNU GPL v3

## No Warranty
**THIS SOFTWARE IS PROVIDED "AS IS" WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED,
INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR
PURPOSE, AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE
LIABLE FOR ANY CLAIM, DAMAGES, OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE
OR OTHER DEALINGS IN THE SOFTWARE.**

## Protection Against Monopolization

### Legal Protection (GPL v3)
The GNU General Public License v3 provides strong legal safeguards:
- **No proprietary forks**: Anyone modifying this software must share changes
- **No exclusive ownership claims**: No entity can claim sole ownership or prevent others from using it
- **Permanent freedom**: The software remains freely available forever
- **No vendor lock-in**: No single organization can control access or create closed versions

### Your Rights as a User
Under GPL v3, you have the right to:
1. Use the software for any lawful purpose
2. Study how it works (source code is available)
3. Modify it to meet your needs
4. Share original or modified versions
5. Share your improvements with others

These rights cannot be revoked or restricted by any entity.

## Project Overview
These programs are designed for selecting participants in sortition processes
for Citizens' and Workers' Branches in democratic systems.

The system consists of two complementary applications:
1. **Roster Generator**: Creates unique roster codes from registration data
2. **Roster Selector**: Performs weighted random selection from registered rosters

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

### Current Architecture
- **Language**: Rust (prototype phase)
- **Storage**: Local Markdown files with roster data
- **Registry Types**: Citizens and Workers registries
- **File Organization**: Date-based folder structure with automatic organization
- **Roster Codes**: 8-character unique identifiers derived from birth dates with collision prevention
- **Selection Limits**: Maximum 4 selections per person, weighted random algorithm

### Two-Application System

#### 1. Roster Generator
- **Purpose**: Creates and manages roster entries
- **Key Features**:
  - Generates unique 8-character roster codes from birth dates
  - Organizes files by date folders (e.g., `citizens-2024-12-20`)
  - Limits to 10 people per file (auto-splits when full)
  - Prevents duplicate roster codes across all files
  - Tracks unique people across entire registry

#### 2. Roster Selector
- **Purpose**: Selects participants from existing rosters
- **Key Features**:
  - Scans ALL files across ALL date folders
  - Weighted random selection (less-selected people have higher chance)
  - Enforces 4-selection maximum per person
  - Updates selection counts in original files
  - Shows available vs. maxed-out statistics

### File Structure

~/Documents/md-data/
├── citizens/ # Citizens registry
│ ├── citizens_001_2024_12_27_143022.md
│ ├── citizens_002_2024_12_28_093045.md
│ └── citizens-2024-12-20/ # Date-based organization
│ ├── citizens_001_2024_12_20_121000.md
│ └── citizens_002_2024_12_20_121500.md
└── workers/ # Workers registry
├── workers_001_2024_12_27_152118.md
└── workers-2024-12-20/
└── workers_001_2024_12_20_131000.md

### File Format
Each roster file contains:
- **Header**: Metadata (registry type, date, counts)
- **Table**: Name, Roster Code, Birth Date, Times Selected
- **Statistics**: File and registry-level statistics
- **Footer**: Generation information and limits

### Data Integrity Features
- **Duplicate Prevention**: Hash-based roster code collision detection
- **Cross-File Validation**: Scans all files/folders for duplicates
- **In-Place Updates**: Selection counts updated directly in source files
- **Automatic Organization**: Date folders for chronological organization

## Installation & Usage

## Prerequisites
- Rust 1.70 or higher
- Standard build tools for your platform

## Building Both Applications

### Clone the repository

```bash
git clone https://codeberg.org/GeaucefStone/Sortition.git
cd Sortition
```

### Build both applications

```bash
# Enter the roster-gen directory
cd roster-gen
cargo build --release

# To return to the previous directory
cd ..

# Enter the roster-selector directory
cd roster-select
cargo build --release
```
