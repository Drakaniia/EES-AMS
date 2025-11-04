# 🎓 Bukidnon State University Portal

A modern, enterprise-grade university login portal built with Next.js 14, TypeScript, and Tailwind CSS.

![License](https://img.shields.io/badge/license-MIT-blue.svg)
![Next.js](https://img.shields.io/badge/Next.js-14-black)
![TypeScript](https://img.shields.io/badge/TypeScript-5-blue)
![Tailwind CSS](https://img.shields.io/badge/Tailwind-3-38bdf8)

## ✨ Features

- 🎨 **Modern UI/UX** - Clean, professional design with diagonal mask effects
- 📱 **Fully Responsive** - Works seamlessly on desktop, tablet, and mobile
- 🔒 **Secure Authentication** - JWT-based auth with role-based access control
- ⚡ **High Performance** - Built with Next.js App Router and Server Components
- ♿ **Accessible** - WCAG 2.1 AA compliant with proper ARIA labels
- 🎭 **Smooth Animations** - Powered by Framer Motion
- 🔍 **Type-Safe** - Full TypeScript support with strict mode
- 📋 **Form Validation** - React Hook Form + Zod for robust validation
- 🎯 **SEO Optimized** - Proper meta tags and structured data

## 🚀 Quick Start

### Prerequisites

- Node.js 18.x or higher
- npm or yarn package manager

### Installation

1. **Clone the repository**

   ```bash
   git clone https://github.com/your-org/university-portal.git
   cd university-portal
   ```

2. **Run the setup script**

   ```bash
   chmod +x setup.sh
   ./setup.sh
   ```

3. **Add your images**
   Place the following in `public/images/`:
   - `campus-background.jpg`
   - `hands-typing.jpg`
   - `bsu-logo.png`

4. **Configure environment**

   ```bash
   cp .env.example .env.local
   # Edit .env.local with your configuration
   ```

5. **Start development server**

   ```bash
   npm run dev
   ```

6. **Open your browser**
   Navigate to [http://localhost:3000/login](http://localhost:3000/login)

## 📁 Project Structure

```
university-portal/
├── src/
│   ├── app/                    # Next.js App Router
│   │   ├── (auth)/            # Auth routes group
│   │   │   └── login/         # Login page
│   │   ├── actions/           # Server actions
│   │   ├── api/               # API routes
│   │   └── layout.tsx         # Root layout
│   ├── components/
│   │   ├── ui/                # Reusable UI components
│   │   ├── forms/             # Form components
│   │   └── layout/            # Layout components
│   ├── lib/
│   │   ├── validations/       # Zod schemas
│   │   ├── auth/              # Auth utilities
│   │   └── utils.ts           # Helper functions
│   └── styles/
│       └── globals.css        # Global styles
├── public/
│   └── images/                # Static images
├── .env.example               # Environment template
└── setup.sh                   # Setup script
```

## 🎨 Design System

### Colors

- **Primary**: `#1e3a5f` (Dark Blue)
- **Primary Foreground**: `#f8fafc` (Light)
- **Background**: `#ffffff` (White)
- **Muted**: `#f1f5f9` (Light Gray)

### Typography

- **Font Family**: Inter (Google Fonts)
- **Heading**: Bold, Uppercase, Tracking Wide
- **Body**: Regular, 14-16px

### Components

All components follow a consistent design pattern:

- Rounded corners (0.5rem)
- Soft shadows for depth
- Smooth transitions
- Focus states for accessibility

## 🔐 Authentication

### User Roles

- **Student** - Access to student portal
- **Faculty** - Access to faculty resources
- **Staff** - Administrative access
- **Admin** - Full system access

### Login Flow

1. User selects role from dropdown
2. Enters User ID and password
3. Form validates input client-side
4. Server action processes authentication
5. JWT token issued on success
6. User redirected to appropriate dashboard

### Security Features

- Password hashing with bcrypt
- JWT token-based sessions
- CSRF protection
- Rate limiting on login attempts
- Secure HTTP-only cookies
- Input sanitization

## 🛠️ Tech Stack

### Core

- **Framework**: Next.js 14 (App Router)
- **Language**: TypeScript 5
- **Styling**: Tailwind CSS 3
- **Form Management**: React Hook Form
- **Validation**: Zod
- **Animation**: Framer Motion

### UI Components

- Custom-built components
- Lucide React icons
- Responsive design patterns

### Development

- **Linting**: ESLint
- **Formatting**: Prettier (optional)
- **Type Checking**: TypeScript strict mode

## 📝 Scripts

```bash
# Development
npm run dev          # Start dev server
npm run build        # Build for production
npm run start        # Start production server
npm run lint         # Run ESLint
npm run type-check   # Check TypeScript types
```

## 🧪 Testing

### Manual Testing

1. Navigate to `/login`
2. Test with demo credentials:
   - User ID: `demo`
   - Password: `password123`
   - Role: Any

### Test Cases

- ✅ Form validation (empty fields)
- ✅ Invalid credentials
- ✅ Successful login
- ✅ Show/hide password
- ✅ Responsive layouts
- ✅ Keyboard navigation
- ✅ Screen reader compatibility

## 🚢 Deployment

### Vercel (Recommended)

```bash
vercel deploy
```

### Docker

```bash
docker build -t university-portal .
docker run -p 3000:3000 university-portal
```

### Environment Variables

Required for production:

- `DATABASE_URL`
- `NEXTAUTH_SECRET`
- `JWT_SECRET`

## 📚 Documentation

- [Integration Guide](./INTEGRATION_GUIDE.md) - Detailed setup instructions
- [API Reference](./docs/API.md) - API documentation
- [Component Library](./docs/COMPONENTS.md) - Component usage guide

## 🤝 Contributing

Contributions are welcome! Please follow these steps:

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Write/update tests
5. Submit a pull request

## 📄 License

Copyright © 2025 Bukidnon State University. All rights reserved.

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 👥 Authors

- Development Team - Bukidnon State University IT Department

## 🙏 Acknowledgments

- Next.js team for the amazing framework
- Vercel for hosting and deployment
- Open source community for the tools and libraries

## 📞 Support

For support, email support@buksu.edu.ph or visit our [support portal](https://support.buksu.edu.ph).

## 🗺️ Roadmap

- [ ] Multi-factor authentication
- [ ] Password reset functionality
- [ ] Self-service registration
- [ ] Social login integration
- [ ] Mobile app companion
- [ ] Biometric authentication
- [ ] Advanced analytics dashboard
- [ ] Internationalization (i18n)

---

Made with ❤️ by Bukidnon State University IT Department
