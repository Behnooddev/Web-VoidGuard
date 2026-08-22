import type { PortDirection } from "@/types/ports";

export type FirewallProtocol = "TCP" | "UDP" | "ANY";
export type FirewallAction = "ALLOW" | "BLOCK";

export interface FirewallRule {
  name: string;
  description: string | null;
  protocol: FirewallProtocol;
  direction: PortDirection;
  action: FirewallAction;
  local_ports: string | null;
  remote_ports: string | null;
  remote_addresses: string | null;
  application_path: string | null;
  enabled: boolean;
  last_seen: string;
}

export interface CreateFirewallRuleRequest {
  name: string;
  description: string | null;
  protocol: FirewallProtocol;
  direction: PortDirection;
  action: FirewallAction;
  local_ports: string | null;
  remote_ports: string | null;
  remote_addresses: string | null;
  application_path: string | null;
  enabled: boolean;
}

export interface SetFirewallRuleEnabledRequest {
  name: string;
  enabled: boolean;
}
