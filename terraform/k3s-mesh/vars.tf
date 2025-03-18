variable "primary_host" {
  type = string
}

variable "local_ip" {
  type = string
}

variable "wg_port" {
  type = string
}

variable "local_nodes" {
  type = map(object({
    private_ip  = string
    internal_ip = string
    port        = string
    primary     = bool
    run_agent   = bool
  }))
}

variable "cloud_node_public_ip" {
  type = string
}

variable "cloud_node_private_ip" {
  type = string
}

variable "cloud_node_instance_id" {
  type = string
}

variable "cloud_depends_on" {
  type = any
}
