variable "node_name" {
  type = string
}

variable "node_type" {
  type = string
}

variable "subnet_id" {
  type = string
}

variable "container_node" {
  type = bool
}

variable "vpc_id" {
  type = string
}

variable "instance_profile_name" {
  type = string
}

variable "base_security_group_id" {
  type = string
}

variable "alb_security_group_id" {
  type = string
}

variable "key_pair_name" {
  type = string
}

variable "deployment" {
  type = string
}


locals {
  # Ubuntu 22.04 server us-west-2
  ubuntu_ami = "ami-008fe2fc65df48dac"
}


resource "aws_eip" "conesnake_primary" {
  network_interface = aws_network_interface.conesnake.id
  domain            = "vpc"

  tags = {
    app = var.deployment
  }
}

resource "aws_network_interface" "conesnake" {
  subnet_id = var.subnet_id

  tags = {
    app = var.deployment
  }
}

resource "aws_network_interface_sg_attachment" "conesnake_base" {
  security_group_id    = var.base_security_group_id
  network_interface_id = aws_network_interface.conesnake.id
}

resource "aws_network_interface_sg_attachment" "conesnake_alb" {
  count                = var.container_node ? 1 : 0
  security_group_id    = var.alb_security_group_id
  network_interface_id = aws_network_interface.conesnake.id
}

resource "aws_instance" "conesnake" {
  ami                  = local.ubuntu_ami
  instance_type        = var.node_type
  iam_instance_profile = var.instance_profile_name
  key_name             = var.key_pair_name

  disable_api_termination = true
  monitoring              = true

  network_interface {
    network_interface_id = aws_network_interface.conesnake.id
    device_index         = 0
  }

  root_block_device {
    volume_size = 10
    volume_type = "gp2"
    encrypted   = true
    tags = {
      app = var.deployment
    }
  }

  credit_specification {
    cpu_credits = "unlimited"
  }

  metadata_options {
    http_tokens                 = "required"
    http_put_response_hop_limit = 2
  }

  tags = {
    Name = var.node_name
    app  = var.deployment
  }
}

output "instance_id" {
  value = aws_instance.conesnake.id
}

output "public_ip" {
  value = aws_eip.conesnake_primary.public_ip
}

output "private_ip" {
  value = aws_network_interface.conesnake.private_ip
}

output "network_interface_id" {
  value = aws_network_interface.conesnake.id
}
