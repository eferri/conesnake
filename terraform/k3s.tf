# Flannel has a wireguard backend, but it doesn't work with NAT
resource "shell_script" "wg_mesh" {
  lifecycle_commands {
    create = file("${path.module}/wg-mesh/create.sh")
    delete = file("${path.module}/wg-mesh/delete.sh")
  }

  depends_on = [
    module.conesnake_primary,
    module.conesnake_relay
  ]

  environment = {
    REMOTE_IP    = var.remote_ip
    REMOTE_NODES = jsonencode(var.workers)
    CLOUD_NODES = jsonencode([
      {
        host        = "primary"
        private_ip  = module.conesnake_primary.private_ip
        public_ip   = module.conesnake_primary.public_ip
        internal_ip = module.conesnake_primary.internal_ip
      },
      {
        host        = "relay"
        private_ip  = module.conesnake_relay.private_ip
        public_ip   = module.conesnake_relay.public_ip
        internal_ip = module.conesnake_relay.internal_ip
      }
    ])
  }

  working_directory = "${path.module}/.."
}


resource "shell_script" "k3s_primary" {
  lifecycle_commands {
    create = file("${path.module}/k3s/create.sh")
    delete = file("${path.module}/k3s/delete.sh")
  }

  depends_on = [
    shell_script.wg_mesh
  ]

  lifecycle {
    replace_triggered_by = [
      shell_script.wg_mesh
    ]
  }

  environment = {
    HOST                = "primary"
    MODE                = "primary"
    PUBLIC_IP           = module.conesnake_primary.public_ip
    INTERNAL_IP         = module.conesnake_primary.internal_ip
    PRIMARY_INTERNAL_IP = module.conesnake_primary.internal_ip
  }

  working_directory = "${path.module}/.."
}

resource "shell_script" "k3s_relay" {
  lifecycle_commands {
    create = file("${path.module}/k3s/create.sh")
    delete = file("${path.module}/k3s/delete.sh")
  }

  depends_on = [
    shell_script.k3s_primary
  ]

  lifecycle {
    replace_triggered_by = [
      shell_script.k3s_primary
    ]
  }


  environment = {
    HOST                = "relay"
    MODE                = "relay"
    PUBLIC_IP           = module.conesnake_relay.public_ip
    INTERNAL_IP         = module.conesnake_relay.internal_ip
    PRIMARY_INTERNAL_IP = module.conesnake_primary.internal_ip
  }

  working_directory = "${path.module}/.."
}


resource "shell_script" "k3s_worker" {
  for_each = { for w in var.workers : w.host => w if w.run_agent }

  lifecycle_commands {
    create = file("${path.module}/k3s/create.sh")
    delete = file("${path.module}/k3s/delete.sh")
  }

  depends_on = [
    shell_script.k3s_primary
  ]

  lifecycle {
    replace_triggered_by = [
      shell_script.k3s_primary
    ]
  }

  environment = {
    HOST                = each.value.host
    MODE                = "worker"
    PUBLIC_IP           = var.remote_ip
    INTERNAL_IP         = each.value.internal_ip
    PRIMARY_INTERNAL_IP = module.conesnake_primary.internal_ip
  }

  working_directory = "${path.module}/.."
}
