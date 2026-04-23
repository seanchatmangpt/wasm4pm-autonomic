/** @type {import('next').NextConfig} */
const nextConfig = {
  reactStrictMode: true,
  transpilePackages: ['three'],
  // Standalone output for Docker / serverless deployments; also reduces
  // image size for matrix-tv shipments.
  output: 'standalone',
};

export default nextConfig;
