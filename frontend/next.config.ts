import type { NextConfig } from "next";

// Where the frontend proxies /api requests. Override in Docker/production via
// the BACKEND_URL environment variable (e.g. http://backend:8080).
const backendUrl = process.env.BACKEND_URL ?? "http://localhost:8080";

const nextConfig: NextConfig = {
  output: "standalone",
  async rewrites() {
    return [
      {
        source: "/api/:path*",
        destination: `${backendUrl}/api/:path*`,
      },
    ];
  },
};

export default nextConfig;
