export type PortProtocol = "TCP" | "UDP";
export type PortRisk = "LOW" | "MEDIUM" | "HIGH" | "UNKNOWN";
export type PortDirection = "INBOUND" | "OUTBOUND";

export interface ListeningPort {
  protocol: PortProtocol;
  local_address: string;
  port: number;
  pid: number | null;
  process_name: string | null;
  executable_path: string | null;
  status: string;
  risk: PortRisk;
  firewall_allowed: boolean | null;
}

export interface PortRuleRequest {
  port: number;
  protocol: PortProtocol;
  direction: PortDirection;
}
