variable "local_ip" {
  type = string
}

variable "ssh_public_key" {
  type = string
}

variable "notification_email" {
  type = string
}

variable "region" {
  type = string
}

variable "zone" {
  type = string
}

variable "project" {
  type = string
}

locals {
  deployment = "conesnake"
  http_port  = 31757
  node_name  = "relay"
  node_type  = "e2-micro"
}

output "cloud_node_public_ip" {
  value = google_compute_instance.conesnake_relay.network_interface.0.access_config.0.nat_ip
}

output "cloud_node_private_ip" {
  value = google_compute_instance.conesnake_relay.network_interface.0.network_ip
}

output "cloud_node_instance_id" {
  value = google_compute_instance.conesnake_relay.instance_id
}
