# Auth Gateway

Central authentication gateway for Bukidnon State University Portal system.

## Features

- Centralized login for all university portals
- Role-based authentication (Student, Faculty, Parent, Admin)
- Secure token-based authentication
- Responsive design with Tailwind CSS
- Modern UI with Framer Motion animations

## Development

```bash
pnpm dev
```

## Build

```bash
pnpm build
```

## Environment Variables

Create `.env.local`:

```env
NEXT_PUBLIC_APP_URL=http://localhost:3000
NEXT_PUBLIC_STUDENT_URL=http://localhost:3002
NEXT_PUBLIC_FACULTY_URL=http://localhost:3003
NEXT_PUBLIC_ADMIN_URL=http://localhost:3004
NEXT_PUBLIC_PARENT_URL=http://localhost:3005
NEXT_PUBLIC_ASSETS_URL=http://localhost:3001
```

## Architecture

This auth gateway serves as the single entry point for authentication across all university portals. After successful login, users are redirected to their respective portal dashboards.