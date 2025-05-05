terraform {
  required_version = "~> 1.11.0"

  backend "gcs" {
    bucket = "conesnake-tf-state"
    prefix = "conesnake-prod"
  }

}

provider "google" {
  project = var.project
  region  = var.region
  zone    = "${var.region}-${var.zone}"
}

provider "google-beta" {
  alias   = "google_beta"
  project = var.project
  region  = var.region
  zone    = "${var.region}-${var.zone}"
}

provider "null" {}

module "gcp" {
  source = "./gcp"

  deployment     = var.deployment
  local_ip       = var.local_ip
  ssh_public_key = var.ssh_public_key
  region         = var.region
  zone           = var.zone
  project        = var.project
  wg_port        = var.wg_port
}

module "k3s_mesh" {
  source                 = "./k3s-mesh"
  primary_host           = var.primary_host
  local_ip               = var.local_ip
  local_nodes            = var.local_nodes
  wg_port                = var.wg_port
  cloud_node_public_ip   = module.gcp.cloud_node_public_ip
  cloud_node_private_ip  = module.gcp.cloud_node_private_ip
  cloud_node_instance_id = module.gcp.cloud_node_instance_id

  cloud_depends_on = module.gcp
}

output "instance_ip" {
  value = module.gcp.cloud_node_public_ip
}
