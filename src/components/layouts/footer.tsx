// src/components/layouts/footer.tsx
import Link from "next/link";

export function Footer() {
  return (
    <footer className="relative z-10 py-8 bg-transparent">
      <div className="container mx-auto px-6">
        <div className="text-center">
          <p className="text-sm text-gray-600 font-normal">
            Copyright © {new Date().getFullYear()} Bukidnon State University.
            All rights reserved.
          </p>

          <div className="mt-4 flex items-center justify-center gap-6">
            <Link
              href="/terms"
              className="text-sm text-gray-600 hover:text-gray-900 transition-colors underline-offset-2 hover:underline"
            >
              Terms of Use
            </Link>
            <span className="text-gray-400">•</span>
            <Link
              href="/privacy"
              className="text-sm text-gray-600 hover:text-gray-900 transition-colors underline-offset-2 hover:underline"
            >
              Privacy Policy
            </Link>
            <span className="text-gray-400">•</span>
            <Link
              href="/contact"
              className="text-sm text-gray-600 hover:text-gray-900 transition-colors underline-offset-2 hover:underline"
            >
              Contact Us
            </Link>
          </div>
        </div>
      </div>
    </footer>
  );
}
