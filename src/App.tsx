import { useEffect, useState } from "react";
import { Routes, Route } from "react-router-dom";
import Sidebar from "@/components/Sidebar";
import Dashboard from "@/pages/Dashboard";
import ProcessesPage from "@/pages/ProcessesPage";
import NetworkPage from "@/pages/NetworkPage";
import ServicesPage from "@/pages/ServicesPage";
import FilesPage from "@/pages/FilesPage";
import StartupPage from "@/pages/StartupPage";
import EventsPage from "@/pages/EventsPage";
import FirewallPage from "@/pages/FirewallPage";
import SecurityPage from "@/pages/SecurityPage";
import ScansPage from "@/pages/ScansPage";
import HealthPage from "@/pages/HealthPage";
import SettingsPage from "@/pages/SettingsPage";
import NotificationManager from "@/components/NotificationManager";
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
      <NotificationManager />
      <Sidebar />
      <main className="flex-1 overflow-y-auto">
        <Routes>
          <Route path="/" element={<Dashboard />} />
          <Route path="/security" element={<SecurityPage />} />
          <Route path="/processes" element={<ProcessesPage />} />
          <Route path="/network" element={<NetworkPage />} />
          <Route path="/firewall" element={<FirewallPage />} />
          <Route path="/services" element={<ServicesPage />} />
          <Route path="/files" element={<FilesPage />} />
          <Route path="/startup" element={<StartupPage />} />
          <Route path="/events" element={<EventsPage />} />
          <Route path="/scans" element={<ScansPage />} />
          <Route path="/health" element={<HealthPage />} />
          <Route
            path="/audit"
            element={<PlaceholderPage title="Audit Log" phase="Phase 4 (audit system UI — backend + DB already live)" />}
          />
          <Route path="/settings" element={<SettingsPage />} />
        </Routes>
      </main>
    </div>
  );
}
