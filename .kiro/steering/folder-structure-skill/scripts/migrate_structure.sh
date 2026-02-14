#!/bin/bash

# Folder Structure Migration Script
# Helps migrate between different folder structure patterns

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Function to show usage
show_usage() {
    echo "Usage: $0 <project_path> <from_pattern> <to_pattern> [options]"
    echo ""
    echo "Patterns:"
    echo "  type-based      - Type-based folder structure"
    echo "  feature-based   - Feature-based folder structure"
    echo "  domain-driven   - Domain-driven folder structure"
    echo "  clean-arch      - Clean architecture"
    echo ""
    echo "Options:"
    echo "  --dry-run       - Show what would be done without making changes"
    echo "  --backup        - Create backup before migration"
    echo "  --verbose       - Verbose output"
    echo ""
    echo "Example:"
    echo "  $0 ./my-project type-based feature-based --backup --verbose"
}

# Function to backup project
backup_project() {
    local project_path="$1"
    local backup_path="${project_path}_backup_$(date +%Y%m%d_%H%M%S)"
    
    print_status "Creating backup at: $backup_path"
    cp -r "$project_path" "$backup_path"
    print_success "Backup created successfully"
    
    echo "$backup_path"
}

# Function to check if directory exists
check_directory() {
    if [ ! -d "$1" ]; then
        print_error "Directory does not exist: $1"
        exit 1
    fi
}

# Function to check for node_modules and other large directories
exclude_patterns() {
    echo "--exclude='node_modules' --exclude='.git' --exclude='dist' --exclude='build' --exclude='.next' --exclude='coverage'"
}

# Function to migrate from type-based to feature-based
migrate_type_to_feature() {
    local src_path="$1"
    local dry_run="$2"
    local verbose="$3"
    
    print_status "Migrating from type-based to feature-based structure"
    
    # Create new directory structure
    local new_dirs=(
        "src/features"
        "src/shared/components"
        "src/shared/hooks"
        "src/shared/services"
        "src/shared/utils"
        "src/shared/types"
        "src/shared/constants"
    )
    
    for dir in "${new_dirs[@]}"; do
        if [ "$dry_run" = "false" ]; then
            mkdir -p "$src_path/$dir"
            [ "$verbose" = "true" ] && print_status "Created directory: $dir"
        else
            print_status "Would create directory: $dir"
        fi
    done
    
    # Migrate files based on naming patterns
    if [ -d "$src_path/src/components" ]; then
        print_status "Analyzing components for feature grouping..."
        
        # Common feature patterns
        declare -A feature_patterns=(
            ["auth"]="auth|login|signup|signin|register|password|reset"
            ["dashboard"]="dashboard|home|main|overview|stats"
            ["user"]="user|profile|account|settings|preferences"
            ["admin"]="admin|manage|management|control"
            ["product"]="product|item|catalog|inventory"
            ["order"]="order|purchase|checkout|cart|payment"
        )
        
        for component in "$src_path/src/components"/*; do
            if [ -f "$component" ]; then
                component_name=$(basename "$component")
                feature="shared"
                
                # Check if component matches any feature pattern
                for feature_name in "${!feature_patterns[@]}"; do
                    pattern="${feature_patterns[$feature_name]}"
                    if echo "$component_name" | grep -iE "($pattern)" > /dev/null; then
                        feature="$feature_name"
                        break
                    fi
                done
                
                # Create feature directory if needed
                feature_dir="src/features/$feature/components"
                if [ "$dry_run" = "false" ]; then
                    mkdir -p "$src_path/$feature_dir"
                    mv "$component" "$src_path/$feature_dir/"
                    [ "$verbose" = "true" ] && print_status "Moved $component_name to $feature_dir"
                else
                    print_status "Would move $component_name to $feature_dir"
                fi
            fi
        done
    fi
    
    # Move other directories to shared
    for dir in "hooks" "services" "utils" "types" "constants"; do
        if [ -d "$src_path/src/$dir" ]; then
            if [ "$dry_run" = "false" ]; then
                mv "$src_path/src/$dir" "$src_path/src/shared/"
                [ "$verbose" = "true" ] && print_status "Moved $dir to shared/"
            else
                print_status "Would move $dir to shared/"
            fi
        fi
    done
}

# Function to update import paths
update_imports() {
    local src_path="$1"
    local dry_run="$2"
    local verbose="$3"
    
    print_status "Updating import paths..."
    
    # Find all TypeScript/JavaScript files
    find "$src_path/src" -name "*.ts" -o -name "*.tsx" -o -name "*.js" -o -name "*.jsx" | while read file; do
        if [ "$verbose" = "true" ]; then
            print_status "Processing $file"
        fi
        
        # Update relative imports
        if [ "$dry_run" = "false" ]; then
            # This is a simplified version - in practice you'd want more sophisticated regex
            sed -i.bak -E "s|from \.\.\/components|from '@/shared/components|g" "$file"
            sed -i.bak -E "s|from \.\.\/hooks|from '@/shared/hooks|g" "$file"
            sed -i.bak -E "s|from \.\.\/utils|from '@/shared/utils|g" "$file"
        else
            print_status "Would update imports in $file"
        fi
    done
}

# Main migration function
perform_migration() {
    local project_path="$1"
    local from_pattern="$2"
    local to_pattern="$3"
    local options=("${@:4}")
    
    local dry_run="false"
    local backup="false"
    local verbose="false"
    
    # Parse options
    for option in "${options[@]}"; do
        case $option in
            --dry-run)
                dry_run="true"
                ;;
            --backup)
                backup="true"
                ;;
            --verbose)
                verbose="true"
                ;;
        esac
    done
    
    check_directory "$project_path"
    
    if [ "$backup" = "true" ] && [ "$dry_run" = "false" ]; then
        backup_path=$(backup_project "$project_path")
    fi
    
    print_status "Starting migration: $from_pattern -> $to_pattern"
    [ "$dry_run" = "true" ] && print_warning "DRY RUN MODE - No changes will be made"
    
    case "$from_pattern->$to_pattern" in
        "type-based->feature-based")
            migrate_type_to_feature "$project_path" "$dry_run" "$verbose"
            ;;
        "type-based->domain-driven")
            print_status "Migration from type-based to domain-driven not yet implemented"
            ;;
        "feature-based->domain-driven")
            print_status "Migration from feature-based to domain-driven not yet implemented"
            ;;
        *)
            print_error "Migration from $from_pattern to $to_pattern not supported"
            exit 1
            ;;
    esac
    
    if [ "$dry_run" = "false" ]; then
        update_imports "$project_path" "$dry_run" "$verbose"
        print_success "Migration completed successfully!"
        [ "$backup" = "true" ] && print_status "Backup available at: $backup_path"
    else
        print_status "Dry run completed. Use without --dry-run to perform migration."
    fi
}

# Parse command line arguments
if [ $# -lt 3 ]; then
    show_usage
    exit 1
fi

PROJECT_PATH="$1"
FROM_PATTERN="$2"
TO_PATTERN="$3"
OPTIONS=("${@:4}")

# Validate patterns
VALID_PATTERNS=("type-based" "feature-based" "domain-driven" "clean-arch")

if [[ ! " ${VALID_PATTERNS[@]} " =~ " ${FROM_PATTERN} " ]]; then
    print_error "Invalid from_pattern: $FROM_PATTERN"
    show_usage
    exit 1
fi

if [[ ! " ${VALID_PATTERNS[@]} " =~ " ${TO_PATTERN} " ]]; then
    print_error "Invalid to_pattern: $TO_PATTERN"
    show_usage
    exit 1
fi

# Perform migration
perform_migration "$PROJECT_PATH" "$FROM_PATTERN" "$TO_PATTERN" "${OPTIONS[@]}"