terraform {
  required_version = "~> 1.3.6"

  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "4.46.0"
    }
    null = {
      source  = "hashicorp/null"
      version = "3.2.1"
    }
  }

  backend "s3" {
    bucket         = "conesnake-tf-state"
    key            = "terraform.tfstate"
    region         = "us-west-2"
    encrypt        = true
    dynamodb_table = "terraform_state"
  }
}

provider "aws" {
  region = "us-west-2"
}

provider "null" {}
