---
name: project-structure-scaffold
description: >
  Automatically creates organized folder structure for any project type and language. Detects project context (frontend, backend, fullstack, CLI, library) and language (JavaScript/TypeScript, Python, Rust, Go, Java, etc.) then creates appropriate architecture. Triggers when user asks to scaffold, structure, or organize a project. This skill is designed for ONE-TIME USE after initial project creation when the folder structure is still unorganized. Creates directories immediately without advisory or explanations.
---

# Project Structure Scaffold

Automatically create well-organized folder structure for any project type and programming language. **ONE-TIME USE**: This skill is meant to be used once after project initialization when the structure is still unorganized.

---

## When to Use This Skill

**Use when:**

- Project was just initialized (e.g., `npm create`, `cargo new`, `django-admin startproject`)
- Folder structure is flat or minimal (only config files, no organized directories)
- User explicitly asks to "scaffold", "organize", or "structure" the project
- First-time setup after project creation

**Do NOT use when:**

- Project already has an organized structure (src/, app/, features/, domain/, etc.)
- Multiple layers of directories already exist
- User is asking to add a single feature or module (not restructure entire project)

---

## Execution Steps

1. **Check if structure is already organized** — Verify the project needs scaffolding:
   - If organized structure exists (src/, app/, features/, domain/, api/, etc.): **STOP and inform user** that structure already exists
   - If only config files and flat structure: **PROCEED**

2. **Detect project context** — Analyze existing files, package managers, and config files to identify:
   - Language: JavaScript/TypeScript, Python, Rust, Go, Java, C#, PHP, Ruby, etc.
   - Project type: Frontend, Backend API, Fullstack, CLI tool, Library/Package, Mobile, Desktop
   - Framework: Next.js, React, FastAPI, Django, Actix, Axum, Express, NestJS, Spring Boot, etc.

3. **Ask user for confirmation** — Before creating folders, briefly show what will be created and ask:
   - "I'll create [architecture pattern] structure for your [language/framework] project. Proceed?"
   - Wait for user confirmation

4. **Create folders immediately** — After confirmation, use file system tools to create the directory structure

5. **Minimal confirmation output** — Only output: "Folder structure created."

---

## Architecture Patterns by Language/Type

### Frontend (JavaScript/TypeScript - React, Vue, Svelte)

```
src/
├── app/          # Application initialization, providers, routing
├── pages/        # Page-level compositions
├── widgets/      # Large composite UI blocks
├── features/     # User-facing features with business logic
├── entities/     # Business entities and domain models
└── shared/       # Reusable utilities, UI kit, API clients
    ├── ui/
    ├── lib/
    └── api/
```

### Next.js (App Router)

```
app/              # Next.js routing (keep at root)
src/
├── features/     # Feature modules
├── entities/     # Domain models
├── shared/       # Shared utilities and components
└── widgets/      # Composite UI blocks
```

### Backend - Python (FastAPI, Django, Flask)

```
src/
├── api/          # API routes/endpoints
│   ├── routes/
│   └── dependencies/
├── core/         # Core business logic
│   ├── domain/   # Domain models
│   ├── services/ # Business services
│   └── use_cases/
├── infrastructure/
│   ├── database/ # Database models, repositories
│   ├── external/ # External API clients
│   └── config/
└── shared/       # Shared utilities, types
tests/
```

### Backend - Rust (Actix, Axum, Rocket)

```
src/
├── api/          # HTTP handlers/routes
│   ├── handlers/
│   └── middleware/
├── domain/       # Domain models and business logic
│   ├── entities/
│   └── services/
├── infrastructure/
│   ├── database/ # Database repositories
│   └── config/
├── shared/       # Shared utilities, errors
└── main.rs
tests/
```

### Backend - Go

```
cmd/
├── api/          # API server entry point
└── cli/          # CLI entry point (if applicable)
internal/
├── api/          # HTTP handlers
├── domain/       # Business logic and models
├── repository/   # Data access layer
├── service/      # Business services
└── config/
pkg/              # Public libraries (if any)
tests/
```

### Backend - Node.js (Express, NestJS, Fastify)

```
src/
├── api/          # Controllers/routes
│   ├── controllers/
│   ├── routes/
│   └── middleware/
├── domain/       # Business logic
│   ├── entities/
│   └── services/
├── infrastructure/
│   ├── database/ # Models, repositories
│   └── external/
├── shared/       # Utilities, types, errors
└── app.ts
tests/
```

### CLI Tool (Any Language)

```
src/
├── commands/     # Command implementations
├── core/         # Core logic
├── utils/        # Utilities
└── main.[ext]
tests/
```

### Library/Package

```
src/
├── core/         # Main library code
├── utils/        # Internal utilities
└── index.[ext]   # Public API
tests/
examples/         # Usage examples
```

---

## Detection Logic

### Language Detection

- Check for: `package.json`, `Cargo.toml`, `go.mod`, `requirements.txt`, `pyproject.toml`, `pom.xml`, `*.csproj`, `Gemfile`, `composer.json`
- Analyze file extensions: `.ts`, `.js`, `.py`, `.rs`, `.go`, `.java`, `.cs`, `.rb`, `.php`

### Framework Detection

- Next.js: `next.config.*` present
- React: `react` in dependencies
- FastAPI/Django: imports in Python files
- Rust web: `actix-web`, `axum`, `rocket` in Cargo.toml
- NestJS: `@nestjs/core` in dependencies
- Express: `express` in dependencies

### Project Type Detection

- Frontend: Has UI framework, no server framework
- Backend: Has server framework, database dependencies
- Fullstack: Both frontend and backend indicators
- CLI: Has CLI framework or binary target
- Library: No application entry point, focused exports

### Structure Organization Check

**Unorganized indicators (proceed with scaffolding):**

- No `src/` directory
- No organized layers (app/, features/, domain/, api/, core/)
- Only config files at root (package.json, Cargo.toml, etc.)
- Flat structure with scattered files

**Organized indicators (do NOT scaffold):**

- `src/` directory exists with subdirectories
- Multiple architectural layers present
- Clear separation of concerns visible

---

## Folder Creation Rules

- Use language-appropriate conventions:
  - JavaScript/TypeScript: kebab-case folders
  - Python: snake_case folders
  - Rust: snake_case folders
  - Go: lowercase folders
  - Java: lowercase folders (packages)
- Create appropriate marker files:
  - Python: `__init__.py` in each package
  - Rust: `mod.rs` or `lib.rs`
  - Go: `.go` files with package declarations
  - JavaScript/TypeScript: `index.ts` barrel exports
- Respect existing structure: Don't overwrite, only add missing layers

- Create test directories matching source structure

---

## Reference

- Read `references/architecture-patterns.md` for detailed pattern information
