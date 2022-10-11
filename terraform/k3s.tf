# Flannel has a wireguard backend, but it doesn't work with NAT

locals {
  cloud_nodes = {
    relay = {
      private_ip  = module.conesnake_relay.private_ip
      public_ip   = module.conesnake_relay.public_ip
      internal_ip = "10.9.1.0"
    }
  }
}

resource "null_resource" "wg_mesh" {
  depends_on = [
    module.conesnake_relay
  ]

  triggers = {
    instance_id        = module.conesnake_relay.instance_id
    create_script_sha  = filesha256("${path.module}/wg-mesh/create_mesh.sh")
    destroy_script_sha = filesha256("${path.module}/wg-mesh/destroy_mesh.sh")

    LOCAL_IP    = var.local_ip
    LOCAL_NODES = jsonencode(var.local_nodes)
    CLOUD_NODES = jsonencode(local.cloud_nodes)
  }

  provisioner "local-exec" {
    environment = self.triggers
    command     = "./wg-mesh/create_mesh.sh"
  }

  provisioner "local-exec" {
    when = destroy

    environment = self.triggers
    command     = "./wg-mesh/destroy_mesh.sh"
  }
}

resource "null_resource" "k3s_primary" {
  depends_on = [
    null_resource.wg_mesh
  ]

  triggers = {
    mesh_id    = null_resource.wg_mesh.id
    script_sha = filesha256("${path.module}/k3s/k3s.sh")

    HOST        = var.primary_host
    INTERNAL_IP = var.local_nodes[var.primary_host].internal_ip
  }

  provisioner "local-exec" {
    environment = self.triggers
    command     = "./terraform/k3s/k3s.sh create primary"
    working_dir = "${path.module}/.."
  }

  provisioner "local-exec" {
    when = destroy

    environment = self.triggers
    command     = "./terraform/k3s/k3s.sh destroy primary"
    working_dir = "${path.module}/.."
  }
}


resource "null_resource" "k3s_relay" {
  depends_on = [
    null_resource.k3s_primary
  ]

  triggers = {
    primary_id = null_resource.k3s_primary.id
    script_sha = filesha256("${path.module}/k3s/k3s.sh")

    HOST                = "relay"
    PUBLIC_IP           = local.cloud_nodes["relay"].public_ip
    INTERNAL_IP         = local.cloud_nodes["relay"].internal_ip
    PRIMARY_HOST        = var.primary_host
    PRIMARY_INTERNAL_IP = var.local_nodes[var.primary_host].internal_ip
  }

  provisioner "local-exec" {
    environment = self.triggers
    command     = "./terraform/k3s/k3s.sh create relay"
    working_dir = "${path.module}/.."
  }

  provisioner "local-exec" {
    when = destroy

    environment = self.triggers
    command     = "./terraform/k3s/k3s.sh destroy relay"
    working_dir = "${path.module}/.."
  }
}


resource "null_resource" "k3s_worker" {
  for_each = { for k, v in var.local_nodes : k => v if !v.primary && v.run_agent }

  depends_on = [
    null_resource.k3s_primary
  ]

  triggers = {
    mesh_id    = null_resource.wg_mesh.id
    script_sha = filesha256("${path.module}/k3s/k3s.sh")

    HOST                = each.key
    INTERNAL_IP         = each.value.internal_ip
    PRIMARY_HOST        = var.primary_host
    PRIMARY_INTERNAL_IP = var.local_nodes[var.primary_host].internal_ip
  }

  provisioner "local-exec" {
    environment = self.triggers
    command     = "./terraform/k3s/k3s.sh create worker"
    working_dir = "${path.module}/.."
  }

  provisioner "local-exec" {
    when = destroy

    environment = self.triggers
    command     = "./terraform/k3s/k3s.sh destroy worker"
    working_dir = "${path.module}/.."
  }
}
