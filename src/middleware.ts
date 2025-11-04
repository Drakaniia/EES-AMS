import { NextResponse } from 'next/server';
import type { NextRequest } from 'next/server';

export async function middleware(_: NextRequest) {
  // Add your middleware logic here
  // Example: Check authentication, redirect, etc.

  return NextResponse.next();
}

export const config = {
  matcher: [
    /*
     * Match all request paths except:
     * - _next/static (static files)
     * - _next/image (image optimization files)
     * - favicon.ico (favicon file)
     * - public folder
     */
    '/((?!_next/static|_next/image|favicon.ico|images).*)',
  ],
};
