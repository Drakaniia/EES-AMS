import { appConfig } from '@/config/app.config';

export function getPortalRedirectUrl(role: string, redirectUrl?: string): string {
  const portals = {
    student: process.env.NEXT_PUBLIC_STUDENT_URL || 'http://localhost:3002',
    faculty: process.env.NEXT_PUBLIC_FACULTY_URL || 'http://localhost:3003',
    admin: process.env.NEXT_PUBLIC_ADMIN_URL || 'http://localhost:3004',
    parent: process.env.NEXT_PUBLIC_PARENT_URL || 'http://localhost:3005',
  };

  const portalUrl = portals[role as keyof typeof portals];
  
  if (!portalUrl) {
    throw new Error(`Unknown role: ${role}`);
  }

  // If there's a specific redirect URL that belongs to correct portal, use it
  if (redirectUrl) {
    try {
      const redirect = new URL(redirectUrl);
      const portal = new URL(portalUrl);
      
      // Check if redirect URL is from the correct portal
      if (redirect.origin === portal.origin) {
        return redirectUrl;
      }
    } catch (error) {
      console.warn('Invalid redirect URL:', redirectUrl);
    }
  }

  // Default to portal dashboard
  return `${portalUrl}/dashboard`;
}