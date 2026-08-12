import { type NextRequest } from "next/server";

// Proxy every /api/* request to the backend. Resolved at request time so the
// same build works wherever BACKEND_URL points (e.g. http://backend:8080 in
// Docker Compose). This replaces a next.config rewrite, whose destination is
// baked at build time and cannot pick up a runtime BACKEND_URL.
const BACKEND_URL = process.env.BACKEND_URL ?? "http://localhost:8080";

export const dynamic = "force-dynamic";

async function proxy(request: NextRequest, path: string[]): Promise<Response> {
  const target = `${BACKEND_URL}/api/${path.join("/")}${request.nextUrl.search}`;

  const headers = new Headers(request.headers);
  headers.delete("host");
  headers.delete("connection");
  headers.delete("content-length");

  const hasBody = request.method !== "GET" && request.method !== "HEAD";
  const init: RequestInit = {
    method: request.method,
    headers,
    body: hasBody ? await request.arrayBuffer() : undefined,
    redirect: "manual",
    cache: "no-store",
  };

  let res: Response;
  try {
    res = await fetch(target, init);
  } catch {
    return new Response(JSON.stringify({ error: "backend unreachable" }), {
      status: 502,
      headers: { "content-type": "application/json" },
    });
  }

  // Strip hop-by-hop/length headers so the runtime can re-encode the body.
  const responseHeaders = new Headers(res.headers);
  responseHeaders.delete("content-encoding");
  responseHeaders.delete("content-length");
  responseHeaders.delete("transfer-encoding");

  return new Response(res.body, {
    status: res.status,
    statusText: res.statusText,
    headers: responseHeaders,
  });
}

type Ctx = { params: Promise<{ path: string[] }> };

export async function GET(request: NextRequest, ctx: Ctx) {
  return proxy(request, (await ctx.params).path);
}
export async function POST(request: NextRequest, ctx: Ctx) {
  return proxy(request, (await ctx.params).path);
}
export async function PUT(request: NextRequest, ctx: Ctx) {
  return proxy(request, (await ctx.params).path);
}
export async function PATCH(request: NextRequest, ctx: Ctx) {
  return proxy(request, (await ctx.params).path);
}
export async function DELETE(request: NextRequest, ctx: Ctx) {
  return proxy(request, (await ctx.params).path);
}
