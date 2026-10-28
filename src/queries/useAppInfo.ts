import { useQuery } from "@tanstack/react-query";
import { api } from "../api";

/** The bundle's name and version; it never changes while the app runs. */
export const useAppInfo = () => useQuery({ queryKey: ["appInfo"], queryFn: api.appInfo });
