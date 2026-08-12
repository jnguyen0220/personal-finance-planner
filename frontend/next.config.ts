import type { NextConfig } from "next";

// /api/* is proxied to the backend at runtime by src/app/api/[...path]/route.ts,
// which reads BACKEND_URL per request. A next.config rewrite can't be used
// because its destination is baked at build time.
const nextConfig: NextConfig = {
  output: "standalone",
};

export default nextConfig;
