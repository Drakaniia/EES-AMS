// src/app/layout.tsx
import type { Metadata, Viewport } from "next";
import { Inter } from "next/font/google";
import "@/styles/globals.css";

const inter = Inter({
  subsets: ["latin"],
  display: "swap",
  variable: "--font-inter",
});

export const metadata: Metadata = {
  title: {
    default: "Bukidnon State University Portal",
    template: "%s | Bukidnon State University",
  },
  description:
    "Official portal for Bukidnon State University students, faculty, and staff",
  keywords: ["university", "education", "bukidnon", "portal", "login"],
  authors: [{ name: "Bukidnon State University" }],
  creator: "Bukidnon State University",
  publisher: "Bukidnon State University",
  formatDetection: {
    telephone: false, // Prevent auto-detection of phone numbers
  },
  appleWebApp: {
    capable: true,
    statusBarStyle: "default",
    title: "BSU Portal",
  },
};

export const viewport: Viewport = {
  width: "device-width",
  initialScale: 1,
  maximumScale: 5, // Allow zooming for accessibility
  userScalable: true,
  viewportFit: "cover", // For devices with notches
  themeColor: [
    { media: "(prefers-color-scheme: light)", color: "#2563eb" },
    { media: "(prefers-color-scheme: dark)", color: "#1e3a8a" },
  ],
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en" suppressHydrationWarning className={inter.variable}>
      <head>
        {/* PWA support */}
        <link rel="manifest" href="/manifest.json" />
        <link rel="apple-touch-icon" href="/icons/icon-192x192.png" />

        {/* Prevent tap highlight on mobile */}
        <style
          dangerouslySetInnerHTML={{
            __html: `
          * {
            -webkit-tap-highlight-color: transparent;
          }
        `,
          }}
        />
      </head>
      <body className={`${inter.className} antialiased`}>{children}</body>
    </html>
  );
}
