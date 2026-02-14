# GitHub Code Review Workflow

This document outlines the GitHub code review workflow for the EES-AMS project, incorporating best practices to ensure high-quality code submissions and efficient collaboration.

## Overview

Code reviews are a critical part of our development process, helping maintain code quality, share knowledge, and catch issues early. This workflow ensures consistent, constructive reviews while respecting everyone's time.

## Pull Request Creation

### Branch Organization

1. Create feature branches from the `develop` branch
2. Use descriptive branch names following the pattern: `feature/description-of-feature`
3. Keep branches focused on a single feature or fix

### Before Creating a PR

1. **Self-Review First**: Review your own changes before requesting reviews
2. **Ensure Tests Pass**: Wait for CI/CD pipeline to complete successfully
3. **Test Your Changes**: Manually test the functionality you've implemented
4. **Check Code Quality**: Run linting and formatting tools locally

### Creating the Pull Request

1. Use clear, descriptive titles following conventional commit format:
   ```
   type(scope): brief description
   ```
   Examples:
   - `feat(student): add Excel import functionality`
   - `fix(auth): resolve JWT token expiration issue`

2. Provide a detailed description with:
   - **Purpose**: What the PR accomplishes
   - **Changes**: Overview of modifications made
   - **Testing**: How you tested these changes
   - **Screenshots**: For UI changes (if applicable)
   - **Related Issues**: Links to relevant issues or tickets

3. Use the PR template in `.github/PULL_REQUEST_TEMPLATE.md`

4. Add appropriate reviewers:
   - Select reviewers based on code areas affected
   - Consider who has time and expertise
   - Don't request too many reviewers (2-3 is ideal)

## Code Review Process

### For Reviewers

#### Review Timeline

- Aim to review PRs within 2 hours of notification during work hours
- If you can't review promptly, communicate expected timeline
- Avoid letting PRs sit for days without review

#### Review Approach

1. **Understand Context**: Read the PR description and related issues
2. **Check Functionality**: Does the code do what it's supposed to do?
3. **Evaluate Quality**:
   - Code follows project conventions
   - Proper error handling
   - Adequate test coverage
   - Documentation updates if needed

4. **Look for Issues**:
   - Potential bugs
   - Security vulnerabilities
   - Performance concerns
   - Maintainability issues

#### Providing Feedback

- **Be Constructive**: Focus on improving the code, not criticizing the author
- **Be Specific**: Clear, actionable suggestions with examples
- **Use Positive Language**: Frame feedback as improvements, not flaws
- **Suggest, Don't Dictate**: Offer alternatives when possible
- **Explain Why**: Help authors understand the reasoning behind suggestions

#### Review Types

1. **Approve**: When PR meets quality standards and is ready to merge
2. **Request Changes**: For issues that must be addressed before merging
3. **Comment**: For suggestions or non-blocking feedback

### For PR Authors

#### Responding to Feedback

1. **Acknowledge All Comments**: Don't leave questions unanswered
2. **Make Changes Promptly**: Address requested changes quickly
3. **Explain Decisions**: If disagreeing with a suggestion, explain why
4. **Push Updates**: Make changes and push to the same branch (no new PR needed)

#### After Changes

1. Mark conversations as resolved
2. Re-request review if substantial changes were made
3. Test again after addressing feedback

## GitHub Features to Utilize

### Draft Pull Requests
- Use for work-in-progress changes that need early feedback
- Clearly mark when a PR is ready for full review
- Helps align efforts early in development

### Inline Comments
- Comment on specific lines of code
- Use suggestions for concrete improvement ideas
- Provide context for why changes are needed

### Protected Branch Rules

Our repository enforces the following rules on protected branches:

1. **Require Pull Request Reviews**: 
   - Minimum of 2 approving reviews
   - Dismiss stale PR approvals when new commits are pushed
   - Required review from code owners (defined in CODEOWNERS file)

2. **Require Status Checks**:
   - All automated tests must pass
   - Code coverage thresholds must be met
   - Linting and formatting checks must pass

3. **Restrict Pushes**:
   - Direct pushes to protected branches are disabled
   - All changes must go through PR process

### CODEOWNERS File

Create a `CODEOWNERS` file to define who owns certain areas of the code:

```
# Global owners
* @team-lead @tech-lead

# Frontend
src/ @frontend-team

# Backend Rust code
src-tauri/src/ @backend-team

# Documentation
docs/ @tech-writer
```

## Automated Quality Checks

### GitHub Actions Workflow

Our PR workflow includes:

1. **Automated Tests**
   - Rust backend tests
   - Frontend tests
   - Integration tests

2. **Code Quality Checks**
   - Rust: cargo clippy, cargo fmt
   - TypeScript: ESLint, Prettier
   - Type checking for frontend

3. **Security Checks**
   - Dependency scanning
   - Code security analysis

4. **Coverage Requirements**
   - Minimum 80% test coverage for new code
   - Coverage reports visible in PR

### Manual Checklist

Reviewers should verify:

- [ ] Code adheres to style guidelines
- [ ] Error handling is appropriate
- [ ] Tests are comprehensive
- [ ] Documentation is updated
- [ ] No hardcoded secrets/keys
- [ ] Performance impact is acceptable
- [ ] Security best practices followed

## Special Cases

### Emergency Fixes

For critical production issues:

1. Create PR with `hotfix/` branch prefix
2. Mark as urgent in PR description
3. Target appropriate reviewers for quick response
4. May be merged with fewer approvals after team discussion

### Large Refactors

1. Break into smaller, reviewable chunks
2. Create tracking issue for overall effort
3. Provide design documentation
4. Consider pairing sessions for complex areas

### External Contributors

1. Provide extra guidance and context
2. Be patient with learning curve
3. Offer detailed explanations
4. Consider pairing on first contributions

## Tooling Recommendations

### Recommended Extensions

- **GitHub Copilot**: For code suggestions and PR summaries
- **GitHub CLI**: For streamlined PR interactions
- **GitLens**: Enhanced Git capabilities in VS Code

### Useful GitHub Features

1. **PR Templates**: Ensure consistent PR information
2. **Issue Templates**: Standardize bug reports and feature requests
3. **Project Boards**: Track PR progress and team workflow
4. **Merge Queues**: For automated merging of approved PRs

## Team Guidelines

### Review Assignment

1. **Primary Reviewer**: Most familiar with code area
2. **Secondary Reviewer**: Broader perspective/QA focus
3. **Rotation**: Distribute reviews across team to spread knowledge

### Meeting Expectations

1. **Daily Standup**: Mention PRs needing urgent review
2. **Weekly Planning**: Schedule time for complex reviews
3. **Retrospective**: Discuss review process improvements

## Metrics and Improvement

### Track Metrics

1. **Time to Review**: Average time from PR creation to first review
2. **Time to Merge**: Average time from PR creation to merge
3. **Review Participation**: Who is reviewing and how often
4. **PR Size**: Distribution of small, medium, large PRs

### Continuous Improvement

1. Quarterly review of review process
2. Adjust based on team feedback
3. Update documentation as process evolves
4. Share lessons learned in team meetings

## Conclusion

This code review workflow ensures quality, knowledge sharing, and team collaboration. By following these guidelines, we maintain high standards while fostering a positive, productive development environment.

Remember: Code reviews are about improving the codebase together, not about gatekeeping. Be constructive, be respectful, and help build better software as a team.