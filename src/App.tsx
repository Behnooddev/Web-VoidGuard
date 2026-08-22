import { useEffect, useState } from "react";
import { Routes, Route } from "react-router-dom";
import Sidebar from "@/components/Sidebar";
import Dashboard from "@/pages/Dashboard";
import ProcessesPage from "@/pages/ProcessesPage";
import NetworkPage from "@/pages/NetworkPage";
import ServicesPage from "@/pages/ServicesPage";
import PlaceholderPage from "@/pages/PlaceholderPage";

type Theme = "light" | "dark" | "system";

function useTheme() {
  const [theme, setTheme] = useState<Theme>(
    () => (localStorage.getItem("theme") as Theme) || "system"
  );

  useEffect(() => {
    const root = document.documentElement;
    const apply = (t: Theme) => {
      const isDark =
        t === "dark" ||
        (t === "system" && window.matchMedia("(prefers-color-scheme: dark)").matches);
      root.classList.toggle("dark", isDark);
    };
    apply(theme);
    localStorage.setItem("theme", theme);

    if (theme === "system") {
      const mq = window.matchMedia("(prefers-color-scheme: dark)");
      const listener = () => apply("system");
      mq.addEventListener("change", listener);
      return () => mq.removeEventListener("change", listener);
    }
  }, [theme]);

  return { theme, setTheme };
}

export default function App() {
  useTheme();

  return (
    <div className="flex h-screen w-screen overflow-hidden bg-background">
      <Sidebar />
      <main className="flex-1 overflow-y-auto">
        <Routes>
          <Route path="/" element={<Dashboard />} />
          <Route
            path="/security"
            element={<PlaceholderPage title="Security" phase="Phase 5 (security scoring)" />}
          />
          <Route path="/processes" element={<ProcessesPage />} />
          <Route path="/network" element={<NetworkPage />} />
          <Route
            path="/firewall"
            element={<PlaceholderPage title="Firewall" phase="Phase 4 (firewall management)" />}
          />
          <Route path="/services" element={<ServicesPage />} />
          <Route
            path="/files"
            element={<PlaceholderPage title="Files" phase="Phase 3 (file integrity monitoring)" />}
          />
          <Route
            path="/startup"
            element={<PlaceholderPage title="Startup" phase="Phase 3 (startup & persistence monitoring)" />}
          />
          <Route
            path="/events"
            element={<PlaceholderPage title="Events" phase="Phase 3 (event engine, full event center UI)" />}
          />
          <Route
            path="/scans"
            element={<PlaceholderPage title="Scans" phase="Phase 5 (scanning system)" />}
          />
          <Route
            path="/health"
            element={<PlaceholderPage title="Health" phase="Phase 5 (health center)" />}
          />
          <Route
            path="/audit"
            element={<PlaceholderPage title="Audit Log" phase="Phase 4 (audit system UI \u2014 backend + DB already live)" />}
          />
          <Route
            path="/settings"
            element={<PlaceholderPage title="Settings" phase="Phase 6 (configuration management)" />}
          />
        </Routes>
      </main>
    </div>
  );
}
