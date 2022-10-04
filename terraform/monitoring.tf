resource "aws_cloudwatch_metric_alarm" "conesnake" {
  alarm_name          = "${local.deployment}-healthy"
  alarm_description   = "${local.deployment}-healthy"
  comparison_operator = "GreaterThanOrEqualToThreshold"
  threshold           = "1"
  evaluation_periods  = "1"

  alarm_actions             = [aws_sns_topic.conesnake.arn]
  insufficient_data_actions = [aws_sns_topic.conesnake.arn]

  metric_query {
    id          = "query"
    label       = "UnHealthyHostCount"
    return_data = "true"

    metric {
      dimensions = {
        TargetGroup  = aws_lb_target_group.conesnake.arn_suffix
        LoadBalancer = aws_lb.conesnake.arn_suffix
      }

      metric_name = "UnHealthyHostCount"
      namespace   = "AWS/ApplicationELB"
      stat        = "Maximum"
      period      = "60"
    }
  }

  tags = {
    app = local.deployment
  }
}

resource "aws_sns_topic" "conesnake" {
  name = local.deployment

  tags = {
    app = local.deployment
  }
}

resource "aws_sns_topic_subscription" "conesnake" {
  topic_arn = aws_sns_topic.conesnake.arn
  endpoint  = var.notification_email
  protocol  = "email"
}
