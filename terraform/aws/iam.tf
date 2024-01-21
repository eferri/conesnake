resource "aws_iam_role" "conesnake" {
  name = local.deployment

  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Action = "sts:AssumeRole"
        Effect = "Allow"
        Principal = {
          Service = "ec2.amazonaws.com"
        }
      },
    ]
  })

  tags = {
    app = local.deployment
  }
}

resource "aws_iam_role_policy_attachment" "conesnake" {
  role       = aws_iam_role.conesnake.id
  policy_arn = aws_iam_policy.conesnake.arn
}

resource "aws_iam_user" "conesnake" {
  name = "conesnake"

  tags = {
    app = local.deployment
  }
}

resource "aws_iam_access_key" "conesnake" {
  user    = aws_iam_user.conesnake.name
  pgp_key = filebase64("${path.module}/../../iam-public-key.gpg")
}

resource "aws_iam_user_policy_attachment" "conesnake" {
  user       = aws_iam_user.conesnake.name
  policy_arn = aws_iam_policy.conesnake.arn
}

resource "aws_iam_policy" "conesnake" {
  name = local.deployment

  policy = jsonencode(
    {
      Version = "2012-10-17",
      Statement = [
        {
          Effect = "Allow",
          Action = [
            "ecr:GetAuthorizationToken",
            "ecr:BatchCheckLayerAvailability",
            "ecr:GetDownloadUrlForLayer",
            "ecr:GetRepositoryPolicy",
            "ecr:DescribeRepositories",
            "ecr:ListImages",
            "ecr:DescribeImages",
            "ecr:BatchGetImage",
            "ecr:GetLifecyclePolicy",
            "ecr:GetLifecyclePolicyPreview",
            "ecr:ListTagsForResource",
            "ecr:DescribeImageScanFindings"
          ],
          Resource = "*"
        }
      ]
    }
  )

  tags = {
    app = local.deployment
  }
}

