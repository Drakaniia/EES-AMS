import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  // Enable standalone output for Docker production builds
  output: 'standalone',
  
  reactStrictMode: true,
  poweredByHeader: false,
  
  // Disable experimental features that might cause permission issues on Windows
  serverExternalPackages: [],

  // Configure Turbopack root to silence warning
  turbopack: {
    root: require('path').resolve(__dirname, '../../../'),
  },


  // Optimize images
  images: {
    formats: ["image/avif", "image/webp"],
    minimumCacheTTL: 60,
    remotePatterns: [
      // GitHub Pages (for assets repository)
      {
        protocol: "https",
        hostname: "drakaniia.github.io",
        pathname: "/university-portal-assets/**",
      },
      {
        protocol: "https",
        hostname: "*.github.io",
        pathname: "/**",
      },
      // Vercel deployments
      {
        protocol: "https",
        hostname: "university-portal-assets.vercel.app",
        pathname: "/**",
      },
      {
        protocol: "https",
        hostname: "*.vercel.app",
        pathname: "/**",
      },
      // Netlify deployments
      {
        protocol: "https",
        hostname: "*.netlify.app",
        pathname: "/**",
      },
      // Local assets server (including Docker internal network)
      {
        protocol: "http",
        hostname: "localhost",
        port: "3001",
        pathname: "/**",
      },
      {
        protocol: "http",
        hostname: "assets",
        port: "3001",
        pathname: "/**",
      },
      {
        protocol: "http",
        hostname: "localhost",
        port: "3002",
        pathname: "/**",
      },
      // Placeholder images
      {
        protocol: "https",
        hostname: "placehold.co",
        pathname: "/**",
      },
    ],
    // Allow unoptimized for development
    unoptimized: process.env.NODE_ENV === "development",
  },

  // Environment variables
  env: {
    NEXT_PUBLIC_APP_URL: process.env.NEXT_PUBLIC_APP_URL,
    NEXT_PUBLIC_ASSETS_URL: process.env.NEXT_PUBLIC_ASSETS_URL,
  },
};

export default nextConfig;
