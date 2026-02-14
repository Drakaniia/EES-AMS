#!/usr/bin/env python3
"""
Folder Structure Generator

This script generates folder structures based on different architectural patterns.
It helps developers quickly scaffold well-organized project structures.
"""

import os
import argparse
from pathlib import Path
from typing import Dict, List

# Define templates for different structure patterns
TEMPLATES = {
    "type-based": {
        "description": "Simple type-based structure for small projects",
        "structure": {
            "src": {
                "components": {},
                "hooks": {},
                "services": {},
                "utils": {},
                "styles": {},
                "assets": {"images": {}, "icons": {}, "fonts": {}},
            }
        },
    },
    "feature-based": {
        "description": "Feature-based structure for medium projects",
        "structure": {
            "src": {
                "features": {
                    "auth": {
                        "components": {},
                        "hooks": {},
                        "services": {},
                        "types": {},
                        "__tests__": {},
                    },
                    "dashboard": {
                        "components": {},
                        "hooks": {},
                        "services": {},
                        "types": {},
                        "__tests__": {},
                    },
                },
                "shared": {
                    "components": {},
                    "hooks": {},
                    "services": {},
                    "utils": {},
                    "types": {},
                    "constants": {},
                },
                "assets": {"images": {}, "icons": {}, "fonts": {}},
                "styles": {"globals": {}, "themes": {}},
            }
        },
    },
    "domain-driven": {
        "description": "Domain-driven structure for large projects",
        "structure": {
            "src": {
                "domains": {
                    "user": {
                        "entities": {},
                        "services": {},
                        "repositories": {},
                        "components": {},
                        "types": {},
                    },
                    "product": {
                        "entities": {},
                        "services": {},
                        "repositories": {},
                        "components": {},
                        "types": {},
                    },
                },
                "shared": {
                    "kernel": {"entities": {}, "services": {}, "events": {}},
                    "infrastructure": {"database": {}, "external": {}, "config": {}},
                    "presentation": {"components": {}, "hooks": {}, "utils": {}},
                },
                "application": {"use_cases": {}, "interfaces": {}, "services": {}},
                "interfaces": {"web": {}, "api": {}, "cli": {}},
            }
        },
    },
    "clean-architecture": {
        "description": "Clean architecture for enterprise projects",
        "structure": {
            "src": {
                "domain": {
                    "entities": {},
                    "services": {},
                    "repositories": {},
                    "errors": {},
                    "events": {},
                },
                "infrastructure": {
                    "database": {"repositories": {}, "migrations": {}, "seeds": {}},
                    "external": {},
                    "config": {},
                    "middleware": {},
                },
                "application": {
                    "commands": {},
                    "queries": {},
                    "handlers": {},
                    "use_cases": {},
                    "interfaces": {},
                },
                "presentation": {
                    "controllers": {},
                    "components": {},
                    "middleware": {},
                    "routes": {},
                },
            }
        },
    },
}


def create_directory_structure(base_path: str, structure: Dict, verbose: bool = False):
    """
    Recursively create directory structure from dictionary

    Args:
        base_path: Base path where to create the structure
        structure: Dictionary representing the folder structure
        verbose: Whether to print verbose output
    """
    base = Path(base_path)

    for name, content in structure.items():
        path = base / name

        if isinstance(content, dict):
            # Create directory
            path.mkdir(parents=True, exist_ok=True)
            if verbose:
                print(f"Created directory: {path}")

            # Create index file for certain directories
            if name in [
                "components",
                "hooks",
                "services",
                "utils",
                "types",
                "features",
                "domains",
            ]:
                index_file = path / "index.ts"
                if not index_file.exists():
                    index_file.write_text(f"// Auto-generated index file for {name}\n")
                    if verbose:
                        print(f"Created file: {index_file}")

            # Recursively create subdirectories
            create_directory_structure(str(path), content, verbose)
        elif isinstance(content, str):
            # Create file with content
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(content)
            if verbose:
                print(f"Created file: {path}")


def generate_package_json(project_name: str, project_type: str):
    """Generate package.json based on project type"""

    base_dependencies = {"typecript": "latest", "@types/node": "latest"}

    if project_type in ["type-based", "feature-based"]:
        dependencies = {
            **base_dependencies,
            "react": "latest",
            "react-dom": "latest",
            "@types/react": "latest",
            "@types/react-dom": "latest",
        }
    elif project_type == "clean-architecture":
        dependencies = {
            **base_dependencies,
            "express": "latest",
            "@types/express": "latest",
            "reflect-metadata": "latest",
        }
    else:
        dependencies = base_dependencies

    dev_dependencies = {
        "eslint": "latest",
        "prettier": "latest",
        "@typescript-eslint/eslint-plugin": "latest",
        "@typescript-eslint/parser": "latest",
    }

    package_json = {
        "name": project_name,
        "version": "1.0.0",
        "description": f"Generated {project_type} project structure",
        "main": "src/index.ts",
        "scripts": {
            "build": "tsc",
            "dev": "ts-node src/index.ts",
            "start": "node dist/index.js",
            "test": "jest",
            "lint": "eslint src/**/*.ts",
            "format": "prettier --write src/**/*.{ts,tsx}",
        },
        "dependencies": dependencies,
        "devDependencies": dev_dependencies,
        "keywords": ["generated", "folder-structure"],
        "author": "",
        "license": "MIT",
    }

    import json

    return json.dumps(package_json, indent=2)


def generate_tsconfig(project_type: str):
    """Generate TypeScript configuration"""

    base_config = {
        "compilerOptions": {
            "target": "ES2020",
            "module": "commonjs",
            "lib": ["ES2020"],
            "outDir": "./dist",
            "rootDir": "./src",
            "strict": True,
            "esModuleInterop": True,
            "skipLibCheck": True,
            "forceConsistentCasingInFileNames": True,
            "resolveJsonModule": True,
            "declaration": True,
            "declarationMap": True,
            "sourceMap": True,
        },
        "include": ["src/**/*"],
        "exclude": ["node_modules", "dist", "**/*.test.ts", "**/*.spec.ts"],
    }

    # Add path mapping for complex structures
    if project_type in ["feature-based", "domain-driven", "clean-architecture"]:
        base_config["compilerOptions"]["baseUrl"] = "."
        base_config["compilerOptions"]["paths"] = {"@/*": ["src/*"]}

    import json

    return json.dumps(base_config, indent=2)


def main():
    parser = argparse.ArgumentParser(description="Generate folder structures")
    parser.add_argument("project_name", help="Name of the project")
    parser.add_argument(
        "structure_type",
        choices=TEMPLATES.keys(),
        help="Type of folder structure to generate",
    )
    parser.add_argument("--path", default=".", help="Base path for project creation")
    parser.add_argument("--verbose", "-v", action="store_true", help="Verbose output")
    parser.add_argument("--config", action="store_true", help="Generate config files")

    args = parser.parse_args()

    if args.structure_type not in TEMPLATES:
        print(f"Invalid structure type. Choose from: {', '.join(TEMPLATES.keys())}")
        return

    template = TEMPLATES[args.structure_type]
    project_path = os.path.join(args.path, args.project_name)

    print(
        f"Generating {args.structure_type} structure for project: {args.project_name}"
    )
    print(f"Description: {template['description']}")
    print(f"Path: {project_path}")

    # Create directory structure
    create_directory_structure(project_path, template["structure"], args.verbose)

    # Generate config files if requested
    if args.config:
        # Package.json
        package_json_content = generate_package_json(
            args.project_name, args.structure_type
        )
        with open(os.path.join(project_path, "package.json"), "w") as f:
            f.write(package_json_content)

        # tsconfig.json
        tsconfig_content = generate_tsconfig(args.structure_type)
        with open(os.path.join(project_path, "tsconfig.json"), "w") as f:
            f.write(tsconfig_content)

        # README.md
        readme_content = f"""# {args.project_name}

Generated using {args.structure_type} folder structure pattern.

## Structure
```
{args.structure_type}
```

## Development

```bash
npm install
npm run dev
```

## Build

```bash
npm run build
```

## Test

```bash
npm test
```
"""
        with open(os.path.join(project_path, "README.md"), "w") as f:
            f.write(readme_content)

        if args.verbose:
            print(
                "Generated configuration files: package.json, tsconfig.json, README.md"
            )

    print(f"✅ Project structure generated successfully!")
    print(f"📁 Path: {project_path}")

    # Show the generated structure
    print("\n📂 Generated structure:")

    def print_tree(directory, prefix=""):
        items = sorted(
            [item for item in os.listdir(directory) if not item.startswith(".")]
        )
        for i, item in enumerate(items):
            item_path = os.path.join(directory, item)
            is_last = i == len(items) - 1
            current_prefix = "└── " if is_last else "├── "
            print(f"{prefix}{current_prefix}{item}")
            if os.path.isdir(item_path) and not item.startswith("."):
                next_prefix = prefix + ("    " if is_last else "│   ")
                print_tree(item_path, next_prefix)

    print_tree(project_path)


if __name__ == "__main__":
    main()
