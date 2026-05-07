import React from "react"
import ReactDOM from "react-dom/client"
import App from "./App"
import { QueryClientProvider } from "@tanstack/react-query"
import { queryClient } from "@/lib/query"
import { Toaster } from "@/components/ui/sonner"
import "./index.css"

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <QueryClientProvider client={queryClient}>
      <App />
      <Toaster />
    </QueryClientProvider>
  </React.StrictMode>
)