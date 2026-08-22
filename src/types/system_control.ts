export interface NetworkAdapter {
  /** GUID identifying this adapter — pass this (never `name`) to changeDns. */
  adapter_id: string;
  name: string;
  description: string;
  adapter_type: "Ethernet" | "Wi-Fi" | "VPN" | "Loopback" | "Other";
  status: "Up" | "Down" | "Unknown";
  mac_address: string | null;
  ipv4_addresses: string[];
  ipv6_addresses: string[];
  gateway: string | null;
  dns_servers: string[];
  dhcp_enabled: boolean | null;
  link_speed_mbps: number | null;
}

export type ServiceStatus =
  | "RUNNING"
  | "STOPPED"
  | "PAUSED"
  | "START_PENDING"
  | "STOP_PENDING"
  | "UNKNOWN";

export type StartupType =
  | "AUTOMATIC"
  | "AUTOMATIC_DELAYED"
  | "MANUAL"
  | "DISABLED"
  | "UNKNOWN";

export interface ServiceInfo {
  name: string;
  display_name: string;
  status: ServiceStatus;
  startup_type: StartupType;
  executable: string | null;
  description: string | null;
  protected: boolean;
}

export type ServiceAction = "START" | "STOP" | "RESTART";

export interface ServiceActionRequest {
  service_name: string;
  action: ServiceAction;
}

export interface ChangeStartupTypeRequest {
  service_name: string;
  startup_type: StartupType;
}

export type DnsMode = "DHCP" | "STATIC";

export interface DnsSettingsRequest {
  adapter_id: string;
  mode: DnsMode;
  /** Required when mode is STATIC. */
  primary_dns: string | null;
  secondary_dns: string | null;
}
