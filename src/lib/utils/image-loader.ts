/**
 * Image Loader Utility
 * Handles image URLs from assets CDN or local fallback
 */

export function getAssetUrl(path: string): string {
  const assetsUrl = process.env.NEXT_PUBLIC_ASSETS_URL;
  
  // Remove leading slash if present
  const cleanPath = path.startsWith('/') ? path.slice(1) : path;
  
  // If assets URL is configured, use it
  if (assetsUrl) {
    // Ensure the assets URL doesn't end with a slash
    const baseUrl = assetsUrl.endsWith('/') ? assetsUrl.slice(0, -1) : assetsUrl;
    return `${baseUrl}/public/${cleanPath}`;
  }
  
  // Fallback to local path
  return `/${cleanPath}`;
}

// Example usage:
// getAssetUrl('images/campus-background.jpg')
// Returns: 'https://drakaniia.github.io/university-portal-assets/public/images/campus-background.jpg'