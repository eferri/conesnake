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

variable "wg_port" {
  type = number
}

locals {
  deployment = "conesnake"
  http_port  = 31757
  node_name  = "relay"
  node_type  = "t3a.micro"
  # Ubuntu 22.04 server us-west-2
  ubuntu_ami = "ami-008fe2fc65df48dac"
}

output "cloud_node_public_ip" {
  value = aws_instance.conesnake.public_ip
}

output "cloud_node_private_ip" {
  value = aws_instance.conesnake.private_ip
}

output "cloud_node_instance_id" {
  value = aws_instance.conesnake.id
}

output "conesnake_access_key_id" {
  value = aws_iam_access_key.conesnake.id
}

output "conesnake_secret_access_key" {
  value = aws_iam_access_key.conesnake.encrypted_secret
}
