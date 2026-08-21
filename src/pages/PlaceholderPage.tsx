import EmptyState from "@/components/EmptyState";

interface PlaceholderPageProps {
  title: string;
  phase: string;
}

/**
 * Per the project's implementation rule: never fake a working
 * feature. Pages for subsystems not yet built (process manager,
 * network center, firewall, DNS, file integrity, startup monitor,
 * event center, scans, health, audit log UI, settings) render this
 * honest "not yet implemented" state instead of mock data, and say
 * exactly which phase will add them.
 */
export default function PlaceholderPage({ title, phase }: PlaceholderPageProps) {
  return (
    <div className="p-6">
      <h1 className="text-xl font-semibold mb-4">{title}</h1>
      <div className="rounded-lg border border-border bg-card">
        <EmptyState
          title="Not implemented yet"
          description={`This module is planned for ${phase}. The command boundary and database tables already exist; the Windows adapter and UI are next.`}
        />
      </div>
    </div>
  );
}
