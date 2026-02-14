---
name: folder-structure-organization
description: Comprehensive guide for organizing scalable application folder structures with separation of concerns. Use when creating new projects, refactoring existing codebases, or establishing architectural standards for teams. Helps prevent codebase bloat as applications grow by implementing high-level folder separation and modular design patterns.
---

# Folder Structure Organization for Scalable Applications

## Quick Start

Choose your approach based on project size and complexity:

**Small Projects** (< 10 components): Use **Type-Based Structure**
**Medium Projects** (10-50 components): Use **Feature-Based Structure**  
**Large Projects** (50+ components): Use **Domain-Driven Structure**

## Core Principles

1. **Separation of Concerns** - Keep different aspects of your app separate
2. **Scalability First** - Structure should accommodate growth
3. **Predictable Organization** - Make it easy to find files
4. **Team Collaboration** - Enable multiple developers to work without conflicts
5. **Consistent Naming** - Use clear, conventional naming patterns

## Structure Patterns

### 1. Type-Based Structure (Small Projects)

```
src/
├── components/     # Reusable UI components
├── hooks/         # Custom React hooks
├── services/      # API calls and external services
├── utils/         # Utility functions
├── styles/        # Global styles and themes
└── assets/        # Static assets (images, fonts)
```

**Use when**: Simple applications, prototypes, learning projects

### 2. Feature-Based Structure (Medium Projects)

```
src/
├── features/
│   ├── auth/
│   │   ├── components/
│   │   ├── hooks/
│   │   ├── services/
│   │   └── index.js
│   ├── dashboard/
│   │   ├── components/
│   │   ├── hooks/
│   │   ├── services/
│   │   └── index.js
│   └── profile/
├── shared/
│   ├── components/    # Cross-feature reusable components
│   ├── hooks/         # Shared custom hooks
│   ├── services/      # Common services
│   └── utils/         # Shared utilities
├── assets/            # Static assets
└── styles/            # Global styles
```

**Use when**: Applications with distinct features, team collaboration

### 3. Domain-Driven Structure (Large Projects)

```
src/
├── domains/
│   ├── user/
│   │   ├── entities/      # Business entities
│   │   ├── services/      # Domain services
│   │   ├── repositories/  # Data access interfaces
│   │   └── components/    # UI components
│   ├── product/
│   │   ├── entities/
│   │   ├── services/
│   │   ├── repositories/
│   │   └── components/
│   └── order/
├── shared/
│   ├── kernel/           # Core business logic
│   ├── infrastructure/   # External implementations
│   └── presentation/     # Common UI elements
├── application/
│   ├── use-cases/        # Application orchestration
│   └── interfaces/       # External interfaces
└── interfaces/           # Framework-specific code
```

**Use when**: Complex business domains, multiple teams, enterprise applications

### 4. Clean Architecture (Enterprise Projects)

```
src/
├── domain/              # Business logic core
│   ├── entities/        # Domain entities
│   ├── services/        # Domain services
│   ├── repositories/    # Repository interfaces
│   └── errors/          # Domain errors
├── infrastructure/      # Technical implementation
│   ├── database/        # Data persistence
│   ├── external/        # External services
│   └── config/          # Configuration
├── application/         # Application coordination
│   ├── commands/        # Use case commands
│   ├── handlers/        # Request handlers
│   └── services/        # Application services
└── presentation/        # UI/API layer
    ├── controllers/     # Request controllers
    ├── components/      # UI components
    └── middleware/      # Cross-cutting concerns
```

**Use when**: Complex enterprise applications, strict architecture requirements

## Implementation Guidelines

### Folder-by-Folder Breakdown

#### `/components`
- **Purpose**: Reusable UI building blocks
- **Types**: Presentational, container, layout
- **Organization**: Group by feature or type
- **Example**: `Button/`, `Modal/`, `Forms/`

#### `/features` or `/domains`
- **Purpose**: Self-contained business functionality
- **Contents**: Components, hooks, services, types
- **Pattern**: Each feature/domain is independent
- **Benefit**: Parallel development, easier refactoring

#### `/shared`
- **Purpose**: Cross-cutting reusable code
- **Contains**: Utils, constants, types, hooks
- **Rule**: No feature-specific logic
- **Example**: `api/`, `validation/`, `constants/`

#### `/services`
- **Purpose**: Business logic and external communication
- **Types**: API clients, data transformation, business rules
- **Pattern**: Separated from UI components
- **Benefit**: Testability, reusability

#### `/hooks`
- **Purpose**: Reusable stateful logic
- **Scope**: Component behavior, data fetching
- **Naming**: Prefix with `use`
- **Example**: `useAuth`, `useApi`, `useLocalStorage`

#### `/utils`
- **Purpose**: Pure utility functions
- **Characteristics**: Stateless, testable, reusable
- **Examples**: Date formatting, validation, helpers
- **Rule**: No side effects

### Naming Conventions

#### Folders
- **kebab-case** for folders: `user-profile`, `auth-service`
- **Descriptive names**: `components` not `comp`, `services` not `svc`
- **Consistent across project**

#### Files
- **PascalCase** for components: `UserProfile.tsx`
- **camelCase** for utilities: `formatDate.js`
- **kebab-case** for assets: `logo-primary.svg`

#### Index Files
- Use `index.ts/js` for clean imports
- Export related functionality
- Example: `features/auth/index.ts`

### Import Path Strategies

#### Absolute Imports
```typescript
// Configure in tsconfig.json or webpack
import { Button } from '@/components/ui/Button';
import { useAuth } from '@/hooks/useAuth';
```

#### Relative Imports
```typescript
// Use within feature modules
import { LoginForm } from './components/LoginForm';
import { authServices } from '../services';
```

#### Barrel Exports
```typescript
// features/auth/index.ts
export { LoginForm } from './components/LoginForm';
export { useAuth } from './hooks/useAuth';
export { authService } from './services/authService';
```

## Scaling Strategies

### 1. Progressive Migration

Start simple, evolve as needed:
```typescript
// Phase 1: Type-based
src/components/
src/hooks/
src/utils/

// Phase 2: Add features
src/features/auth/
src/features/dashboard/

// Phase 3: Extract shared
src/shared/
src/common/
```

### 2. Code Splitting by Feature

```typescript
// Dynamic imports for features
const AuthFeature = lazy(() => import('./features/auth'));
const DashboardFeature = lazy(() => import('./features/dashboard'));
```

### 3. Micro-Frontend Boundaries

```
src/
├── shell/              # Application shell
├── mf-auth/           # Auth micro-frontend
├── mf-dashboard/      # Dashboard micro-frontend
└── shared/            # Shared utilities
```

## Best Practices

### Do's
- ✅ Group related code together
- ✅ Keep feature modules self-contained
- ✅ Use consistent naming conventions
- ✅ Document folder structure decisions
- ✅ Review and refactor as project grows
- ✅ Separate UI from business logic
- ✅ Create clear boundaries between modules

### Don'ts
- ❌ Create deeply nested folders
- ❌ Mix concerns in single folders
- ❌ Use ambiguous names like `stuff`, `misc`
- ❌ Create circular dependencies
- ❌ Put everything in one folder
- ❌ Ignore structure as project scales
- ❌ Mix presentational and business logic

## Testing Organization

### Collocated Tests
```
components/
├── Button/
│   ├── Button.tsx
│   ├── Button.test.tsx
│   └── Button.stories.tsx
```

### Test Directories
```
src/
├── components/
└── tests/
    ├── components/
    ├── integration/
    └── e2e/
```

### Recommendation
- **Small projects**: Collocated tests
- **Large projects**: Separate test directories
- **Always**: Mirror source structure in tests

## Migration Checklist

### Assess Current State
- [ ] Identify current pain points
- [ ] List all features/domains
- [ ] Analyze dependencies between modules
- [ ] Map out ideal target structure

### Plan Migration
- [ ] Choose appropriate structure pattern
- [ ] Define migration phases
- [ ] Create folder structure plan
- [ ] Update build configuration

### Execute Migration
- [ ] Create new folder structure
- [ ] Move files incrementally
- [ ] Update import paths
- [ ] Run tests after each move
- [ ] Update documentation

### Verify
- [ ] All tests pass
- [ ] Build succeeds
- [ ] Team understands new structure
- [ ] Documentation updated

## Tools and Configuration

### ESLint Rules for Imports
```json
{
  "rules": {
    "import/no-cycle": "error",
    "import/no-self-import": "error",
    "import/no-useless-path-segments": "error"
  }
}
```

### TypeScript Path Mapping
```json
{
  "compilerOptions": {
    "baseUrl": ".",
    "paths": {
      "@/*": ["src/*"],
      "@/components/*": ["src/components/*"],
      "@/features/*": ["src/features/*"]
    }
  }
}
```

### IDE Configuration
- Configure folder exclusions
- Set up code navigation shortcuts
- Enable file nesting for related files

## Common Mistakes to Avoid

1. **Over-engineering** - Don't use complex patterns for simple projects
2. **Inconsistent structure** - Maintain the same pattern throughout
3. **Ignoring growth** - Plan for scale from the beginning
4. **Mixing concerns** - Keep business logic separate from UI
5. **Deep nesting** - More than 3-4 levels is too deep
6. **Vague naming** - Use descriptive, specific names

## Resources

- [Feature-Sliced Design](https://feature-sliced.design/)
- [Clean Architecture](https://blog.cleancoder.com/uncle-bob/2012/08/13/the-clean-architecture.html)
- [Domain-Driven Design](https://en.wikipedia.org/wiki/Domain-driven_design)

## Decision Framework

Use this checklist to choose the right structure:

### Project Complexity
- **Simple**: Type-based structure
- **Moderate**: Feature-based structure
- **Complex**: Domain-driven structure
- **Enterprise**: Clean architecture

### Team Size
- **1-3 developers**: Simple structures work well
- **4-10 developers**: Feature-based approach
- **10+ developers**: Domain-driven with clear boundaries

### Business Domain
- **Single domain**: Feature-based approach
- **Multiple domains**: Domain-driven structure
- **Complex relationships**: Clean architecture

By following these patterns and principles, your application will remain maintainable and scalable as it grows.