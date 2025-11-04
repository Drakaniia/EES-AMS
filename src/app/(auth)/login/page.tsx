// src/app/(auth)/login/page.tsx
import { Metadata } from "next";
import Image from "next/image";
import { LoginForm } from "@/components/features/auth/login-form";
import { Header } from "@/components/layouts/header";
import { Footer } from "@/components/layouts/footer";

export const metadata: Metadata = {
  title: "Login | Bukidnon State University",
  description: "Sign in to the Bukidnon State University Portal",
};

// Helper function to get asset URLs
const getAssetUrl = (path: string) => {
  const assetsUrl = process.env.NEXT_PUBLIC_ASSETS_URL;
  if (!assetsUrl) {
    console.warn("NEXT_PUBLIC_ASSETS_URL is not configured.");
    return null;
  }

  const cleanPath = path.startsWith("/") ? path.slice(1) : path;
  const baseUrl = assetsUrl.endsWith("/") ? assetsUrl.slice(0, -1) : assetsUrl;

  // Assets are served from /images, /documents, etc. (not /public)
  return `${baseUrl}/${cleanPath}`;
};

export default function LoginPage() {
  const campusImageUrl = getAssetUrl("images/campus/campus-background.jpg");
  const handsTypingUrl = getAssetUrl("images/ui/hands-typing.jpg");
  const logoUrl = getAssetUrl("images/campus/bsu-logo.png");

  return (
    <div className="relative flex flex-col min-h-screen overflow-x-hidden bg-gray-50">
      {/* Full Background Image - Visible on all screen sizes with blur and brightness */}
      {campusImageUrl && (
        <div className="fixed inset-0 z-0 pointer-events-none">
          <Image
            src={campusImageUrl}
            alt="Bukidnon State University Campus"
            fill
            className="object-cover"
            style={{
              filter: "blur(8px) saturate(0.7) brightness(1.4)",
            }}
            priority
            unoptimized
          />
          <div className="absolute inset-0 bg-white/30" />
        </div>
      )}

      {/* Header */}
      <Header />

      {/* Main Content */}
      <main className="relative z-10 flex-1 flex items-center justify-center px-4 py-8 sm:px-6 lg:px-8 mt-4 lg:mt-10">
        <div className="w-full max-w-5xl">
          {/* Card Container */}
          <div className="bg-white rounded-lg shadow-lg lg:shadow-elevated overflow-hidden">
            <div className="grid lg:grid-cols-[1fr_1fr]">
              {/* Left Section (Image) */}
              <div className="relative bg-gradient-to-br from-gray-800 to-gray-600 h-48 sm:h-64 lg:h-auto diagonal-clip-responsive">
                <div className="absolute inset-0 diagonal-clip-responsive-inner">
                  {handsTypingUrl ? (
                    <Image
                      src={handsTypingUrl}
                      alt="Student typing on laptop"
                      fill
                      className="object-cover"
                      sizes="(max-width: 1024px) 100vw, 50vw"
                      priority
                      unoptimized
                    />
                  ) : (
                    <div className="w-full h-full bg-gradient-to-br from-blue-900 to-blue-700" />
                  )}
                  <div className="absolute inset-0 bg-black/30" />
                </div>

                {/* University Branding */}
                <div className="relative z-10 p-4 sm:p-6 lg:p-8">
                  <div className="flex items-start gap-3">
                    <div className="flex-shrink-0 w-15 h-15 sm:w-12 sm:h-12 lg:w-19 lg:h-15 flex items-center justify-center">
                      {logoUrl ? (
                        <Image
                          src={logoUrl}
                          alt="BSU Logo"
                          width={96}
                          height={96}
                          className="object-contain"
                          unoptimized
                        />
                      ) : (
                        <div className="text-white text-2xl font-bold">BSU</div>
                      )}
                    </div>
                    <div className="text-white flex-1 min-w-0">
                      <h2 className="text-sm sm:text-base lg:text-lg font-bold uppercase tracking-wide leading-tight">
                        Bukidnon State University
                      </h2>
                      <p className="text-xs font-light opacity-90 leading-relaxed mt-0.5">
                        Malaybalay City, Bukidnon
                      </p>
                    </div>
                  </div>
                </div>
              </div>

              {/* Right Section (Login Form) - White Background */}
              <div className="bg-white p-6 sm:p-8 lg:p-12 flex items-center min-h-[600px]">
                <div className="w-full">
                  <LoginForm />
                </div>
              </div>
            </div>
          </div>
        </div>
      </main>

      {/* Footer */}
      <Footer />
    </div>
  );
}
