# Husky Git Hooks

This directory contains Git hooks managed by Husky to improve code quality and ensure consistent commit messages.

## Available Hooks

### pre-commit
Runs linting and formatting on staged files:
- ESLint with auto-fix for TypeScript/JavaScript files
- Prettier formatting for all supported file types

### commit-msg
Validates commit messages according to conventional commit format:
- `feat:` for new features
- `fix:` for bug fixes
- `chore:` for maintenance tasks
- `refactor:` for code refactoring
- And other conventional commit types

### pre-push
Runs build to ensure code compiles before pushing to remote

## Usage

### Normal workflow
```bash
git add .
git commit -m "feat: add new authentication feature"  # Pre-commit and commit-msg hooks run
git push  # Pre-push hook runs
```

### Bypassing hooks (use sparingly)
```bash
git commit -m "wip: temporary commit" --no-verify
git push --no-verify
```

## Dependencies
- Husky - Git hooks manager
- lint-staged - Run linters on staged files
- ESLint - JavaScript/TypeScript linter
- Prettier - Code formatter
- @commitlint - Conventional commit message validation