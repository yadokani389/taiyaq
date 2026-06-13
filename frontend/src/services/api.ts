import createClient from "openapi-fetch";
import type { paths } from "../types/api";

export const getBaseURL = () => {
  const baseURL = import.meta.env.VITE_API_BASE_URL;
  if (!baseURL) {
    throw new Error(
      "VITE_API_BASE_URL is not defined in environment variables",
    );
  }
  return baseURL.replace(/\/+$/, "");
};

export const openApiClient = createClient<paths>({
  baseUrl: getBaseURL(),
});
