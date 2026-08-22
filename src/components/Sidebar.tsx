import { useState } from "react";
import { NavLink } from "react-router-dom";
import {
  ShieldCheck,
  LayoutDashboard,
  Cpu,
  Network,
  Flame,
  Cog,
  Files,
  Rocket,
  ListTree,
  ScanLine,
  HeartPulse,
  ScrollText,
  Settings,
  ChevronsLeft,
  ChevronsRight,
} from "lucide-react";
import { cn } from "@/lib/utils";

const NAV_ITEMS = [
  { to: "/", label: "Dashboard", icon: LayoutDashboard, end: true },
  { to: "/security", label: "Security", icon: ShieldCheck },
  { to: "/processes", label: "Processes", icon: Cpu },
  { to: "/network", label: "Network", icon: Network },
  { to: "/firewall", label: "Firewall", icon: Flame },
  { to: "/services", label: "Services", icon: Cog },
  { to: "/files", label: "Files", icon: Files },
  { to: "/startup", label: "Startup", icon: Rocket },
  { to: "/events", label: "Events", icon: ListTree },
  { to: "/scans", label: "Scans", icon: ScanLine },
  { to: "/health", label: "Health", icon: HeartPulse },
  { to: "/audit", label: "Audit Log", icon: ScrollText },
  { to: "/settings", label: "Settings", icon: Settings },
];

export default function Sidebar() {
  const [collapsed, setCollapsed] = useState(false);

  return (
    <aside
      className={cn(
        "h-screen shrink-0 border-r border-border bg-card flex flex-col transition-all duration-200",
        collapsed ? "w-[68px]" : "w-[236px]"
      )}
    >
      <div className="flex items-center gap-2 px-4 h-14 border-b border-border">
        <ShieldCheck className="h-6 w-6 text-primary shrink-0" />
        {!collapsed && (
          <span className="font-semibold tracking-tight text-sm">VoidGuard</span>
        )}
      </div>

      <nav className="flex-1 overflow-y-auto py-2 px-2" aria-label="Primary">
        {NAV_ITEMS.map((item) => (
          <NavLink
            key={item.to}
            to={item.to}
            end={item.end}
            title={item.label}
            className={({ isActive }) =>
              cn(
                "flex items-center gap-3 rounded-md px-3 py-2 text-sm mb-0.5 transition-colors outline-none",
                "focus-visible:ring-2 focus-visible:ring-primary",
                isActive
                  ? "bg-accent text-primary font-medium"
                  : "text-muted-foreground hover:bg-muted hover:text-foreground"
              )
            }
          >
            <item.icon className="h-4 w-4 shrink-0" />
            {!collapsed && <span className="truncate">{item.label}</span>}
          </NavLink>
        ))}
      </nav>

      <button
        onClick={() => setCollapsed((c) => !c)}
        className="flex items-center justify-center h-10 border-t border-border text-muted-foreground hover:text-foreground hover:bg-muted transition-colors"
        aria-label={collapsed ? "Expand sidebar" : "Collapse sidebar"}
      >
        {collapsed ? (
          <ChevronsRight className="h-4 w-4" />
        ) : (
          <ChevronsLeft className="h-4 w-4" />
        )}
      </button>
    </aside>
  );
}
