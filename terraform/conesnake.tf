locals {
  deployment = "conesnake"
}

resource "aws_iam_instance_profile" "conesnake" {
  name = local.deployment
  role = aws_iam_role.conesnake.name

  tags = {
    app = local.deployment
  }
}

resource "aws_ecr_repository" "conesnake" {
  name                 = "conesnake"
  image_tag_mutability = "MUTABLE"
  force_delete         = true
}

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
    Name                     = local.deployment
    "kubernetes.io/role/elb" = "1"
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

resource "aws_security_group" "alb" {
  name        = "alb"
  description = "alb"
  vpc_id      = aws_vpc.conesnake.id

  ingress {
    description = "https"
    from_port   = 59213
    to_port     = 59213
    protocol    = "tcp"
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

resource "aws_security_group" "conesnake_base" {
  name        = "conesnake_base"
  description = "conesnake_base"
  vpc_id      = aws_vpc.conesnake.id

  ingress {
    description = "ssh"
    from_port   = 22
    to_port     = 22
    protocol    = "tcp"
    cidr_blocks = ["${var.remote_ip}/32"]
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


resource "aws_security_group" "conesnake_alb" {
  name        = "conesnake_alb"
  description = "conesnake_alb"
  vpc_id      = aws_vpc.conesnake.id

  ingress {
    description     = "alb"
    from_port       = 8080
    to_port         = 8080
    protocol        = "tcp"
    security_groups = [aws_security_group.alb.id]
  }

  tags = {
    app = local.deployment
  }
}


# Infrastructure

data "aws_route53_zone" "conesnake" {
  name         = var.domain
  private_zone = false
}

resource "aws_acm_certificate" "conesnake" {
  domain_name       = var.domain
  validation_method = "DNS"

  tags = {
    app = local.deployment
  }

  lifecycle {
    create_before_destroy = true
  }
}

resource "aws_route53_record" "conesnake" {
  for_each = {
    for dvo in aws_acm_certificate.conesnake.domain_validation_options : dvo.domain_name => {
      name   = dvo.resource_record_name
      record = dvo.resource_record_value
      type   = dvo.resource_record_type
    }
  }

  allow_overwrite = true
  name            = each.value.name
  records         = [each.value.record]
  ttl             = 60
  type            = each.value.type
  zone_id         = data.aws_route53_zone.conesnake.zone_id
}

resource "aws_acm_certificate_validation" "conesnake" {
  certificate_arn         = aws_acm_certificate.conesnake.arn
  validation_record_fqdns = [for record in aws_route53_record.conesnake : record.fqdn]
}

resource "aws_lb" "conesnake" {
  name               = local.deployment
  internal           = false
  load_balancer_type = "application"
  security_groups    = [aws_security_group.alb.id]
  subnets            = [for subnet in aws_subnet.conesnake : subnet.id]

  tags = {
    app = local.deployment
  }
}

resource "aws_lb_listener" "conesnake" {
  load_balancer_arn = aws_lb.conesnake.arn
  port              = 59213
  protocol          = "HTTPS"
  ssl_policy        = "ELBSecurityPolicy-2016-08"
  certificate_arn   = aws_acm_certificate.conesnake.arn

  default_action {
    type             = "forward"
    target_group_arn = aws_lb_target_group.conesnake.arn
  }
}

resource "aws_lb_target_group" "conesnake" {
  name        = local.deployment
  port        = 8080
  protocol    = "HTTP"
  vpc_id      = aws_vpc.conesnake.id
  target_type = "instance"

  health_check {
    interval            = "10"
    timeout             = 2
    healthy_threshold   = 2
    unhealthy_threshold = 2
    path                = "/"
    matcher             = "200"
  }
}

resource "aws_key_pair" "conesnake" {
  key_name   = local.deployment
  public_key = var.ssh_public_key

  tags = {
    app = local.deployment
  }
}


module "conesnake_primary" {
  source                 = "./node"
  node_name              = "primary"
  node_type              = "t3.small"
  vpc_id                 = aws_vpc.conesnake.id
  subnet_id              = aws_subnet.conesnake["a"].id
  container_node         = false
  instance_profile_name  = aws_iam_instance_profile.conesnake.name
  base_security_group_id = aws_security_group.conesnake_base.id
  alb_security_group_id  = aws_security_group.conesnake_alb.id
  key_pair_name          = aws_key_pair.conesnake.key_name
  deployment             = local.deployment
  internal_ip            = "10.9.1.0"
}


module "conesnake_relay" {
  source                 = "./node"
  node_name              = "relay"
  node_type              = "t2.micro"
  vpc_id                 = aws_vpc.conesnake.id
  subnet_id              = aws_subnet.conesnake["a"].id
  container_node         = true
  target_group_arn       = aws_lb_target_group.conesnake.arn
  instance_profile_name  = aws_iam_instance_profile.conesnake.name
  base_security_group_id = aws_security_group.conesnake_base.id
  alb_security_group_id  = aws_security_group.conesnake_alb.id
  key_pair_name          = aws_key_pair.conesnake.key_name
  deployment             = local.deployment
  internal_ip            = "10.9.1.1"
}


output "conesnake_target_group_arn" {
  value = aws_lb_target_group.conesnake.arn
}
