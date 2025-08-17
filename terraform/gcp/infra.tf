
resource "google_project_service" "artifact_registry" {
  project            = var.project
  service            = "artifactregistry.googleapis.com"
  disable_on_destroy = false
}

resource "google_project_service" "compute_engine" {
  project            = var.project
  service            = "compute.googleapis.com"
  disable_on_destroy = false
}

# Artifact Registry

resource "google_service_account" "conesnake_registry" {
  account_id   = "${var.deployment}-registry-saccount"
  display_name = "${var.deployment}_registry_service_account"
}

resource "google_artifact_registry_repository" "conesnake" {
  provider      = google-beta
  project       = var.project
  location      = var.region
  repository_id = "conesnake"
  format        = "DOCKER"

  cleanup_policy_dry_run = false

  cleanup_policies {
    id     = "keep-one"
    action = "KEEP"
    condition {
      tag_state = "TAGGED"
    }
  }

  cleanup_policies {
    id     = "delete-old"
    action = "DELETE"

    condition {
      tag_state  = "ANY"
      older_than = "60s"
    }
  }

  labels = {
    app = var.deployment
  }
}

# Network

resource "google_compute_network" "vpc_network" {
  name                    = "my-vpc-network"
  auto_create_subnetworks = false
}

resource "google_compute_subnetwork" "conesnake_subnet" {
  name          = "${var.deployment}-subnet"
  ip_cidr_range = "10.8.0.0/24"
  network       = google_compute_network.vpc_network.id
}


resource "google_compute_firewall" "http" {
  name    = "http"
  network = google_compute_network.vpc_network.id

  allow {
    protocol = "tcp"
    ports    = [local.http_port]
  }

  target_tags = [var.deployment]

  source_ranges = ["0.0.0.0/0"]
}

resource "google_compute_firewall" "ssh" {
  name    = "ssh"
  network = google_compute_network.vpc_network.id

  allow {
    protocol = "tcp"
    ports    = ["22"]
  }

  target_tags = [var.deployment]

  source_ranges = ["${var.local_ip}/32"]
}

resource "google_compute_firewall" "wireguard" {
  name    = "wireguard"
  network = google_compute_network.vpc_network.id

  allow {
    protocol = "udp"
    ports    = [var.wg_port]
  }

  target_tags = [var.deployment]

  source_ranges = ["0.0.0.0/0"]
}

resource "google_compute_address" "conesnake_instance" {
  name = "${var.deployment}-instance"
}

# GCE Instances

resource "google_service_account" "conesnake_instance" {
  account_id   = "${var.deployment}-instance-saccount"
  display_name = "${var.deployment}_instance_service_account"
}

resource "google_compute_instance" "conesnake_relay" {
  name         = local.node_name
  machine_type = local.node_type
  zone         = "${var.region}-${var.zone}"

  tags = [var.deployment]

  allow_stopping_for_update = true

  boot_disk {
    initialize_params {
      image = "ubuntu-os-cloud/ubuntu-minimal-2404-noble-amd64-v20250710"

      type = "pd-standard"

      labels = {
        app = var.deployment
      }
    }
  }

  network_interface {
    network    = google_compute_network.vpc_network.id
    subnetwork = google_compute_subnetwork.conesnake_subnet.id

    access_config {
      nat_ip = google_compute_address.conesnake_instance.address
    }
  }

  metadata = {
    ssh-keys = "ubuntu:${var.ssh_public_key}"
  }

  service_account {
    email  = google_service_account.conesnake_instance.email
    scopes = ["cloud-platform"]
  }

  labels = {
    app = var.deployment
  }
}
