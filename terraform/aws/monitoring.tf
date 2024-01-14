resource "aws_route53_health_check" "conesnake" {
  ip_address        = aws_instance.conesnake.public_ip
  port              = local.http_port
  type              = "HTTP"
  resource_path     = "/ping"
  failure_threshold = "2"
  request_interval  = "30"

  regions = [
    "us-west-2",
    "us-west-1",
    "us-east-1"
  ]

  tags = {
    Name = "conesnake-health-check"
    app  = local.deployment
  }
}

resource "aws_cloudwatch_metric_alarm" "conesnake" {
  provider = aws.us_east_1

  alarm_name          = "${local.deployment}-healthy"
  alarm_description   = "${local.deployment}-healthy"
  comparison_operator = "LessThanThreshold"
  threshold           = "1"
  evaluation_periods  = "1"

  alarm_actions             = [aws_sns_topic.conesnake.arn]
  insufficient_data_actions = [aws_sns_topic.conesnake.arn]
  ok_actions                = [aws_sns_topic.conesnake.arn]

  dimensions = {
    HealthCheckId = aws_route53_health_check.conesnake.id
  }

  metric_name = "HealthCheckStatus"
  namespace   = "AWS/Route53"
  statistic   = "Minimum"
  period      = 60

  tags = {
    app = local.deployment
  }
}

resource "aws_sns_topic" "conesnake" {
  provider = aws.us_east_1

  name = local.deployment

  tags = {
    app = local.deployment
  }
}

resource "aws_sns_topic_subscription" "conesnake" {
  provider = aws.us_east_1

  topic_arn = aws_sns_topic.conesnake.arn
  endpoint  = var.notification_email
  protocol  = "email"
}
