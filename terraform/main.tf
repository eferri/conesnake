terraform {
  required_version = "~> 1.6.6"

  backend "s3" {
    bucket         = "conesnake-tf-state"
    key            = "terraform.tfstate"
    region         = "us-west-2"
    encrypt        = true
    dynamodb_table = "terraform_state"
  }
}

provider "aws" {
  region = var.region
}

provider "aws" {
  region = "us-east-1"
  alias  = "us_east_1"
}

provider "google" {
  region = var.region
}

provider "null" {}

module "conesnake_aws" {
  source = "./aws"

  providers = {
    aws.us_east_1 = aws.us_east_1
  }

  primary_host       = var.primary_host
  local_ip           = var.local_ip
  ssh_public_key     = var.ssh_public_key
  pgp_public_key     = filebase64("${path.module}/../iam-public-key.gpg")
  notification_email = var.notification_email
  region             = var.region
}

module "k3s_mesh" {
  source                 = "./k3s-mesh"
  primary_host           = var.primary_host
  local_ip               = var.local_ip
  local_nodes            = var.local_nodes
  cloud_node_public_ip   = module.conesnake_aws.cloud_node_public_ip
  cloud_node_private_ip  = module.conesnake_aws.cloud_node_private_ip
  cloud_node_instance_id = module.conesnake_aws.cloud_node_instance_id

  cloud_depends_on = module.conesnake_aws
}

output "conesnake_access_key_id" {
  value = module.conesnake_aws.conesnake_access_key_id
}

output "conesnake_secret_access_key" {
  value = module.conesnake_aws.conesnake_secret_access_key
}
