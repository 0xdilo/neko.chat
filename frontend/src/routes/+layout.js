import { redirect } from '@sveltejs/kit';
import { browser } from '$app/environment';

const protectedRoutes = ['/', '/settings'];

export async function load({ url }) {
  const currentPath = url.pathname;
  const isProtectedRoute = protectedRoutes.includes(currentPath);
  
  // Only check auth on client side and for protected routes
  if (browser && isProtectedRoute) {
    const token = localStorage.getItem('neko-auth-token');
    if (!token) {
      throw redirect(302, '/auth');
    }
  }
  
  // Don't redirect from /auth on server side - let client handle it
  
  return {
    currentPath,
    isProtectedRoute
  };
}