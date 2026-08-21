export interface ProcessInfo {
  pid: number;
  parent_pid: number | null;
  name: string;
  exe_path: string | null;
  cpu_percent: number;
  memory_bytes: number;
  start_time_unix: number;
  publisher: string | null;
  signed: boolean | null;
  integrity_level: string | null;
  network_connection_count: number | null;
  sha256: string | null;
}

export interface AppError {
  code: string;
  message: string;
  details: string | null;
  recoverable: boolean;
}
