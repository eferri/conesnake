# VPC

resource "aws_vpc" "conesnake" {
  cidr_block = "10.8.0.0/16"

  enable_dns_hostnames = true

  tags = {
    app = local.deployment
  }
}

resource "aws_internet_gateway" "conesnake" {
  vpc_id = aws_vpc.conesnake.id

  tags = {
    app = local.deployment
  }
}

resource "aws_subnet" "conesnake" {
  for_each = {
    a = { cidr : "10.8.0.0/24", az : "us-west-2a" },
    b = { cidr : "10.8.1.0/24", az : "us-west-2b" },
    c = { cidr : "10.8.2.0/24", az : "us-west-2c" }
  }

  depends_on = [
    aws_internet_gateway.conesnake
  ]

  vpc_id                  = aws_vpc.conesnake.id
  cidr_block              = each.value.cidr
  availability_zone       = each.value.az
  map_public_ip_on_launch = true

  tags = {
    Name = local.deployment
  }
}

resource "aws_default_route_table" "conesnake" {
  default_route_table_id = aws_vpc.conesnake.default_route_table_id

  route {
    cidr_block = "0.0.0.0/0"
    gateway_id = aws_internet_gateway.conesnake.id
  }

  route {
    ipv6_cidr_block = "::/0"
    gateway_id      = aws_internet_gateway.conesnake.id
  }

  tags = {
    app = local.deployment
  }
}

# Security groups

resource "aws_security_group" "conesnake_base" {
  name        = "conesnake_base"
  description = "conesnake_base"
  vpc_id      = aws_vpc.conesnake.id

  ingress {
    description = "ssh"
    from_port   = 22
    to_port     = 22
    protocol    = "tcp"
    cidr_blocks = ["${var.local_ip}/32"]
  }

  ingress {
    description = "http"
    from_port   = local.http_port
    to_port     = local.http_port
    protocol    = "tcp"
    cidr_blocks = ["0.0.0.0/0"]
  }

  ingress {
    description = "wireguard"
    from_port   = 59203
    to_port     = 59203
    protocol    = "udp"
    cidr_blocks = ["0.0.0.0/0"]
  }

  egress {
    from_port        = 0
    to_port          = 0
    protocol         = "-1"
    cidr_blocks      = ["0.0.0.0/0"]
    ipv6_cidr_blocks = ["::/0"]
  }

  tags = {
    app = local.deployment
  }
}


# Infrastructure

resource "aws_ecr_repository" "conesnake" {
  name                 = "conesnake"
  image_tag_mutability = "MUTABLE"
  force_delete         = true

  tags = {
    app = local.deployment
  }
}


resource "aws_ecr_lifecycle_policy" "foopolicy" {
  repository = aws_ecr_repository.conesnake.name

  policy = <<EOF
{
  "rules": [
    {
      "rulePriority": 1,
      "description": "delete untagged images",
      "selection": {
        "tagStatus": "untagged",
        "countType": "imageCountMoreThan",
        "countNumber": 1
      },
      "action": {
        "type": "expire"
      }
    }
  ]
}
EOF
}

resource "aws_key_pair" "conesnake" {
  key_name   = local.deployment
  public_key = var.ssh_public_key

  tags = {
    app = local.deployment
  }
}

resource "aws_eip" "conesnake_primary" {
  network_interface = aws_network_interface.conesnake.id
  domain            = "vpc"

  tags = {
    app = local.deployment
  }
}

resource "aws_network_interface" "conesnake" {
  subnet_id = aws_subnet.conesnake["a"].id

  tags = {
    app = local.deployment
  }
}

resource "aws_network_interface_sg_attachment" "conesnake_base" {
  security_group_id    = aws_security_group.conesnake_base.id
  network_interface_id = aws_network_interface.conesnake.id
}

resource "aws_iam_instance_profile" "conesnake" {
  name = local.deployment
  role = aws_iam_role.conesnake.name

  tags = {
    app = local.deployment
  }
}

resource "aws_instance" "conesnake" {
  ami                  = local.ubuntu_ami
  instance_type        = local.node_type
  iam_instance_profile = aws_iam_instance_profile.conesnake.name
  key_name             = aws_key_pair.conesnake.key_name

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
      app = local.deployment
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
    Name = local.node_name
    app  = local.deployment
  }
}
