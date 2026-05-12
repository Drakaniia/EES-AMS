# Architecture Patterns Reference

A catalog of folder structure architectures for any project type and language. Use this file when the scaffold skill needs to select appropriate patterns.

---

## Table of Contents

1. [Frontend Patterns](#frontend-patterns)
   - Feature-Sliced Design (FSD)
   - Module-Based (Feature Folders)
   - Atomic Design
   - Next.js App Router Conventions
2. [Backend Patterns - Node.js/TypeScript](#backend-patterns---nodejstypescript)
   - Layered (N-Tier)
   - Clean Architecture
   - Hexagonal (Ports & Adapters)
   - Domain-Driven Design (DDD)
   - MVC
3. [Backend Patterns - Python](#backend-patterns---python)
   - FastAPI Structure
   - Django Structure
   - Flask Structure
4. [Backend Patterns - Rust](#backend-patterns---rust)
   - Actix/Axum Web Service
   - CLI Application
   - Library/Crate
5. [Backend Patterns - Go](#backend-patterns---go)
   - Standard Go Project Layout
   - Simple Service Layout
6. [Backend Patterns - Other Languages](#backend-patterns---other-languages)
   - Java/Spring Boot
   - C#/.NET
   - PHP/Laravel
   - Ruby/Rails
7. [CLI & Library Patterns](#cli--library-patterns)
8. [Fullstack / Monorepo Patterns](#fullstack--monorepo-patterns)
9. [Quick Selection Guide](#quick-selection-guide)

---

## Frontend Patterns

### Feature-Sliced Design (FSD)

**Best for**: Medium-to-large React/Vue/Svelte SPAs, teams of 3+, complex business domains.

**Core idea**: Divide by _business domain_ (slices) then by _technical purpose_ (segments). Strict one-directional dependency: upper layers depend on lower, never the reverse.

**Layers** (top → bottom, high to low abstraction):

```
src/
├── app/        # Bootstrap: router, providers, global styles, i18n
├── pages/      # Full page views composed from widgets/features
├── widgets/    # Large self-contained UI composites (Header, Sidebar, Feed)
├── features/   # Product features: actions that deliver business value (auth, cart)
├── entities/   # Business entities: User, Product, Order (model + UI + api)
└── shared/     # Cross-cutting: UI kit, hooks, utils, API client, types, config
```

**Within each slice** (features/, entities/, pages/, widgets/):

```
auth/
├── ui/         # React components
├── model/      # State, hooks, selectors, business logic
├── api/        # API calls / RTK Query / SWR slices
├── lib/        # Helpers specific to this slice
└── index.ts    # Public API (barrel export — the only thing other layers import)
```

**Rules**:

- A module may only import from layers _below_ it
- Slices on the same layer cannot import from each other (use `shared/` instead)
- `index.ts` in each slice = the only public surface (encapsulation)
- `app/` and `shared/` have no slices — just segments

**When NOT to use**: Solo MVPs, simple blogs, apps with < 5 pages. Overhead outweighs benefits.

**Tooling**: `@feature-sliced/eslint-config`, `steiger` linter, FSD CLI

---

### Module-Based (Feature Folders)

**Best for**: Small-to-medium projects, teams of 1–4, MVPs, when FSD feels heavy.

**Core idea**: Group by feature/domain. No strict dependency rules.

```
src/
├── features/
│   ├── auth/
│   │   ├── components/
│   │   ├── hooks/
│   │   ├── services/
│   │   └── types.ts
│   └── dashboard/
├── components/   # Truly shared, reusable UI components
├── hooks/        # Shared hooks
├── utils/        # Shared utilities
├── types/        # Global types
├── services/     # API layer / HTTP client
└── App.tsx
```

**Rules**: Keep features independent. Lift shared code to `components/` or `utils/` only when reused 2+ times.

---

### Atomic Design

**Best for**: Design system / component library projects; UI-heavy teams with a strong design system.

**Core idea**: UI components are atoms → molecules → organisms → templates → pages.

```
src/
├── components/
│   ├── atoms/      # Button, Input, Label, Icon
│   ├── molecules/  # SearchBar (Input + Button), FormField
│   ├── organisms/  # NavBar, ProductCard, UserForm
│   └── templates/  # Page layout shells (no data, just structure)
├── pages/          # Assembled templates with real data
└── ...
```

**Warning**: Atomic Design alone doesn't handle business logic or API layers. Combine with feature folders or FSD for full-stack apps.

---

### Next.js App Router Conventions

**Best for**: Any Next.js 13+ project. Respect framework-prescribed conventions.

```
app/                    # Next.js: file-based routing
├── (auth)/             # Route group (no URL segment)
│   ├── login/
│   │   └── page.tsx
│   └── register/
│       └── page.tsx
├── dashboard/
│   ├── layout.tsx
│   └── page.tsx
├── api/                # Route handlers
│   └── users/
│       └── route.ts
├── globals.css
└── layout.tsx
src/                    # Application logic (parallel to app/)
├── features/           # Feature modules (FSD-style or simple)
├── components/         # Shared UI components
├── hooks/
├── lib/                # Utilities, DB client, auth helpers
└── types/
public/                 # Static assets
```

**Key**: Don't fight the `app/` convention. Put business logic in `src/`, keep `app/` for routing glue.

---

## Backend Patterns - Node.js/TypeScript

### Layered Architecture (N-Tier)

**Best for**: Simple CRUD APIs, REST services, Express/Koa/Fastify with no heavy domain logic.

```
src/
├── controllers/    # HTTP layer: parse request, call service, send response
├── services/       # Business logic / use cases
├── repositories/   # Data access (DB queries)
├── models/         # Data models / schemas (Mongoose, Prisma, TypeORM)
├── middleware/     # Auth, error handling, logging
├── routes/         # Route definitions
├── validators/     # Request validation (Zod, Joi)
├── config/         # Environment, DB config
└── app.ts
```

**Rules**: Controller → Service → Repository. No skipping layers.

---

### Clean Architecture

**Best for**: Medium-to-large backends with growing business logic. Language-agnostic.

**Core idea**: Business rules at the center; frameworks/DBs/HTTP at the outside. Dependency direction always _inward_.

```
src/
├── domain/              # Core business: entities, value objects, domain events
│   ├── entities/
│   ├── value-objects/
│   └── repositories/    # Interfaces only (no implementations)
├── application/         # Use cases / application services
│   ├── use-cases/
│   │   ├── create-order/
│   │   │   ├── CreateOrderUseCase.ts
│   │   │   └── CreateOrderDTO.ts
│   │   └── ...
│   └── ports/           # Input/output interfaces
├── infrastructure/      # Framework + external: DB, HTTP, email, etc.
│   ├── persistence/     # Repository implementations (TypeORM, Prisma, etc.)
│   ├── http/            # Express/Fastify controllers, routes
│   ├── email/
│   └── config/
└── main.ts              # Composition root: wire dependencies
```

**Rules**: `domain/` has zero imports from `application/` or `infrastructure/`. `application/` uses domain interfaces. `infrastructure/` implements them.

---

### Hexagonal Architecture (Ports & Adapters)

**Best for**: Services that must swap out external dependencies (DBs, message queues, APIs). Strong testability.

**Core idea**: The app is a hexagon. Ports = interfaces. Adapters = implementations of those interfaces.

```
src/
├── core/                   # Business logic (domain + application)
│   ├── domain/
│   │   ├── entities/
│   │   └── value-objects/
│   ├── application/
│   │   └── use-cases/
│   └── ports/
│       ├── in/             # Driving ports (what the app exposes)
│       └── out/            # Driven ports (what the app needs)
├── adapters/
│   ├── in/                 # Inbound: HTTP controllers, CLI, gRPC handlers
│   │   └── http/
│   └── out/                # Outbound: DB adapters, email, external APIs
│       ├── persistence/
│       └── messaging/
└── main.ts
```

**Rules**: Adapters depend on ports, never on each other. Core has no knowledge of adapters.

---

### Domain-Driven Design (DDD) — Module per Bounded Context

**Best for**: Complex enterprise domains, microservices, large monoliths with distinct business subdomains.

```
src/
├── modules/
│   ├── orders/             # Bounded context
│   │   ├── domain/
│   │   │   ├── Order.ts    # Aggregate root
│   │   │   ├── OrderItem.ts
│   │   │   └── OrderRepository.ts  # Interface
│   │   ├── application/
│   │   │   └── PlaceOrderUseCase.ts
│   │   ├── infrastructure/
│   │   │   └── TypeORMOrderRepository.ts
│   │   └── presentation/
│   │       └── OrderController.ts
│   ├── inventory/
│   └── users/
├── shared/                 # Shared kernel
│   ├── domain/
│   └── infrastructure/
└── main.ts
```

**DDD Building Blocks**:

- **Entity**: has identity, mutable (User, Order)
- **Value Object**: no identity, immutable (Money, Address)
- **Aggregate**: cluster of entities with one root
- **Repository**: interface for persistence
- **Domain Service**: business logic that spans multiple entities
- **Domain Event**: something significant that happened (OrderPlaced)

---

### MVC (Model-View-Controller)

**Best for**: Traditional web frameworks (Rails, Laravel, Django, Express with server-rendered views).

```
src/
├── models/         # Data models
├── views/          # Templates / server-rendered HTML
├── controllers/    # Request handling
├── routes/
├── middleware/
├── config/
└── app.ts
```

---

## Backend Patterns - Python

### FastAPI Structure

**Best for**: Modern Python REST APIs, async services, microservices.

```
src/
├── api/
│   ├── routes/              # API route handlers
│   │   ├── users.py
│   │   └── items.py
│   ├── dependencies.py      # Dependency injection
│   └── middleware.py
├── core/
│   ├── domain/              # Domain models
│   │   ├── entities/
│   │   └── value_objects/
│   ├── services/            # Business logic
│   └── use_cases/
├── infrastructure/
│   ├── database/
│   │   ├── models.py        # SQLAlchemy/Tortoise models
│   │   ├── repositories.py
│   │   └── session.py
│   ├── external/            # External API clients
│   └── config.py
├── shared/
│   ├── schemas.py           # Pydantic schemas
│   ├── exceptions.py
│   └── utils.py
└── main.py
tests/
requirements.txt
pyproject.toml
```

---

### Django Structure

**Best for**: Full-featured web applications, admin panels, content management.

```
project_name/
├── apps/                    # Django apps (bounded contexts)
│   ├── users/
│   │   ├── models.py
│   │   ├── views.py
│   │   ├── serializers.py
│   │   ├── urls.py
│   │   └── tests.py
│   ├── orders/
│   └── inventory/
├── core/                    # Shared/core functionality
│   ├── middleware.py
│   ├── permissions.py
│   └── utils.py
├── config/                  # Settings
│   ├── settings/
│   │   ├── base.py
│   │   ├── development.py
│   │   └── production.py
│   ├── urls.py
│   └── wsgi.py
├── static/
├── media/
└── manage.py
tests/
requirements/
```

---

### Flask Structure

**Best for**: Lightweight Python APIs, small-to-medium web services.

```
src/
├── api/
│   ├── routes/
│   │   ├── users.py
│   │   └── items.py
│   └── middleware.py
├── models/                  # SQLAlchemy models
├── services/                # Business logic
├── schemas/                 # Marshmallow schemas
├── utils/
├── config.py
└── app.py
tests/
requirements.txt
```

---

## Backend Patterns - Rust

### Actix/Axum Web Service

**Best for**: High-performance web services, REST APIs, microservices.

```
src/
├── api/
│   ├── handlers/            # HTTP request handlers
│   │   ├── users.rs
│   │   └── items.rs
│   ├── middleware/
│   ├── routes.rs            # Route definitions
│   └── mod.rs
├── domain/
│   ├── entities/            # Domain models
│   │   ├── user.rs
│   │   └── item.rs
│   ├── services/            # Business logic
│   │   └── user_service.rs
│   └── mod.rs
├── infrastructure/
│   ├── database/
│   │   ├── models.rs        # Database models (Diesel/SQLx)
│   │   ├── repositories.rs
│   │   └── mod.rs
│   ├── config.rs
│   └── mod.rs
├── shared/
│   ├── errors.rs            # Error types
│   ├── utils.rs
│   └── mod.rs
├── main.rs
└── lib.rs
tests/
Cargo.toml
```

---

### Rust CLI Application

**Best for**: Command-line tools, utilities.

```
src/
├── commands/                # Command implementations
│   ├── init.rs
│   ├── build.rs
│   └── mod.rs
├── core/                    # Core logic
│   ├── config.rs
│   └── mod.rs
├── utils/
│   └── mod.rs
└── main.rs
tests/
Cargo.toml
```

---

### Rust Library/Crate

**Best for**: Reusable libraries, shared code.

```
src/
├── core/                    # Main library code
│   ├── module_a.rs
│   └── module_b.rs
├── utils/
│   └── helpers.rs
└── lib.rs                   # Public API
tests/
examples/                    # Usage examples
├── basic.rs
└── advanced.rs
benches/                     # Benchmarks
Cargo.toml
```

---

## Backend Patterns - Go

### Standard Go Project Layout

**Best for**: Medium-to-large Go services, production applications.

```
cmd/
├── api/                     # API server entry point
│   └── main.go
└── cli/                     # CLI entry point (if applicable)
    └── main.go
internal/                    # Private application code
├── api/
│   ├── handlers/            # HTTP handlers
│   ├── middleware/
│   └── routes.go
├── domain/
│   ├── entities/            # Domain models
│   └── services/            # Business logic
├── repository/              # Data access layer
│   ├── postgres/
│   └── redis/
├── config/
│   └── config.go
└── errors/
    └── errors.go
pkg/                         # Public libraries (optional)
├── logger/
└── validator/
tests/
go.mod
go.sum
```

---

### Simple Go Service Layout

**Best for**: Small Go services, microservices, simple APIs.

```
src/
├── handlers/                # HTTP handlers
├── models/                  # Data models
├── services/                # Business logic
├── repository/              # Database access
├── middleware/
├── config/
│   └── config.go
└── main.go
tests/
go.mod
```

---

## Backend Patterns - Other Languages

### Java/Spring Boot

```
src/
├── main/
│   ├── java/
│   │   └── com/example/app/
│   │       ├── controller/
│   │       ├── service/
│   │       ├── repository/
│   │       ├── model/
│   │       ├── dto/
│   │       ├── config/
│   │       └── Application.java
│   └── resources/
│       ├── application.properties
│       └── static/
└── test/
pom.xml
```

---

### C#/.NET

```
src/
├── Api/                     # Web API project
│   ├── Controllers/
│   ├── Middleware/
│   └── Program.cs
├── Application/             # Business logic
│   ├── Services/
│   └── Interfaces/
├── Domain/                  # Domain models
│   └── Entities/
├── Infrastructure/          # Data access
│   ├── Data/
│   └── Repositories/
└── Shared/
tests/
*.sln
*.csproj
```

---

### PHP/Laravel

```
app/
├── Http/
│   ├── Controllers/
│   ├── Middleware/
│   └── Requests/
├── Models/
├── Services/
└── Repositories/
database/
├── migrations/
└── seeders/
routes/
├── web.php
└── api.php
resources/
├── views/
└── js/
tests/
composer.json
```

---

### Ruby/Rails

```
app/
├── controllers/
├── models/
├── views/
├── services/
├── jobs/
└── mailers/
config/
├── routes.rb
└── database.yml
db/
├── migrate/
└── seeds.rb
lib/
test/
Gemfile
```

---

## CLI & Library Patterns

### CLI Tool (Language Agnostic)

```
src/
├── commands/                # Command implementations
│   ├── init.[ext]
│   ├── build.[ext]
│   └── deploy.[ext]
├── core/                    # Core logic
│   ├── config.[ext]
│   └── executor.[ext]
├── utils/
│   └── helpers.[ext]
└── main.[ext]
tests/
```

---

### Library/Package (Language Agnostic)

```
src/
├── core/                    # Main library code
│   ├── module_a.[ext]
│   └── module_b.[ext]
├── utils/                   # Internal utilities
│   └── helpers.[ext]
└── index.[ext]              # Public API / exports
tests/
examples/                    # Usage examples
docs/
```

---

## Fullstack / Monorepo Patterns

### Turborepo / pnpm Workspaces

```
apps/
├── web/            # Next.js frontend
├── api/            # Express/Fastify backend
└── mobile/         # React Native
packages/
├── ui/             # Shared component library
├── config/         # Shared ESLint, TypeScript configs
├── types/          # Shared TypeScript types
└── utils/          # Shared utility functions
turbo.json
package.json        # Root workspace
```

### Nx Monorepo

```
apps/
├── frontend/
└── backend/
libs/
├── shared-ui/
├── data-access/
└── util/
nx.json
```

---

## Quick Selection Guide

### Frontend

| Scenario                          | Recommended Pattern               |
| --------------------------------- | --------------------------------- |
| Solo MVP / weekend project        | Module-based (Feature Folders)    |
| Small team SPA (React/Vue)        | Module-based or light FSD         |
| Medium+ team SPA                  | Feature-Sliced Design (FSD)       |
| Next.js project                   | Next.js App Router + src/features |
| Design system / component library | Atomic Design                     |

### Backend - Node.js/TypeScript

| Scenario                            | Recommended Pattern          |
| ----------------------------------- | ---------------------------- |
| Simple REST API / CRUD              | Layered (MVC)                |
| Backend with growing business logic | Clean Architecture           |
| Backend needing swappable infra     | Hexagonal (Ports & Adapters) |
| Complex domain / enterprise         | DDD (Bounded Contexts)       |

### Backend - Python

| Scenario                     | Recommended Pattern |
| ---------------------------- | ------------------- |
| Modern async REST API        | FastAPI Structure   |
| Full web app with admin      | Django Structure    |
| Lightweight API/microservice | Flask Structure     |

### Backend - Rust

| Scenario               | Recommended Pattern       |
| ---------------------- | ------------------------- |
| Web service / REST API | Actix/Axum Structure      |
| Command-line tool      | CLI Application Structure |
| Reusable library       | Library/Crate Structure   |

### Backend - Go

| Scenario                  | Recommended Pattern        |
| ------------------------- | -------------------------- |
| Production service        | Standard Go Project Layout |
| Simple API / microservice | Simple Go Service Layout   |

### Backend - Other

| Language/Framework | Recommended Pattern   |
| ------------------ | --------------------- |
| Java/Spring Boot   | Spring Boot Structure |
| C#/.NET            | .NET Structure        |
| PHP/Laravel        | Laravel Structure     |
| Ruby/Rails         | Rails Structure       |

### Other Project Types

| Scenario                    | Recommended Pattern            |
| --------------------------- | ------------------------------ |
| CLI tool (any language)     | CLI Tool Pattern               |
| Library/package             | Library Pattern                |
| Multi-app / shared packages | Monorepo (Turborepo / Nx)      |
| Microservices               | Per-service pattern + monorepo |
