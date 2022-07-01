resource "aws_iam_user" "ci" {
  name = "ci"

  tags = {
    app = "ci"
  }
}

resource "aws_iam_access_key" "ci" {
  user    = aws_iam_user.ci.name
  pgp_key = filebase64("${path.module}/../iam-public-key.gpg")
}

resource "aws_iam_user_policy_attachment" "ci_ecr" {
  user       = aws_iam_user.ci.name
  policy_arn = aws_iam_policy.ci_ecr.arn
}

resource "aws_iam_policy" "ci_ecr" {
  name = "ci_ecr"

  policy = jsonencode(
    {
      Version = "2012-10-17",
      Statement = [
        {
          Effect = "Allow",
          Action = [
            "ecr:PutImageTagMutability",
            "ecr:DescribeImageScanFindings",
            "ecr:StartImageScan",
            "ecr:GetLifecyclePolicyPreview",
            "ecr:GetDownloadUrlForLayer",
            "ecr:PutImageScanningConfiguration",
            "ecr:DescribeImageReplicationStatus",
            "ecr:ListTagsForResource",
            "ecr:UploadLayerPart",
            "ecr:ListImages",
            "ecr:BatchGetRepositoryScanningConfiguration",
            "ecr:PutImage",
            "ecr:UntagResource",
            "ecr:BatchGetImage",
            "ecr:CompleteLayerUpload",
            "ecr:DescribeImages",
            "ecr:TagResource",
            "ecr:DescribeRepositories",
            "ecr:InitiateLayerUpload",
            "ecr:BatchCheckLayerAvailability",
            "ecr:ReplicateImage",
            "ecr:GetRepositoryPolicy",
            "ecr:GetLifecyclePolicy"
          ],
          Resource = "*"
        },
        {
          Effect = "Allow",
          Action = [
            "ecr:GetRegistryPolicy",
            "ecr:BatchImportUpstreamImage",
            "ecr:CreateRepository",
            "ecr:DescribeRegistry",
            "ecr:DescribePullThroughCacheRules",
            "ecr:GetAuthorizationToken",
            "ecr:PutRegistryScanningConfiguration",
            "ecr:CreatePullThroughCacheRule",
            "ecr:GetRegistryScanningConfiguration",
            "ecr:PutReplicationConfiguration"
          ],
          Resource = "*"
        },
      ]
    }
  )
}

output "ci_access_key_id" {
  value = aws_iam_access_key.ci.id
}

output "ci_secret_access_key" {
  value = aws_iam_access_key.ci.encrypted_secret
}
