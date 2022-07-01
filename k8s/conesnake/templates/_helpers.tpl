{{/*
Common labels
*/}}

{{- define "common_labels" -}}
app.kubernetes.io/instance: {{ .Release.Name }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
app.kubernetes.io/name: {{ .Chart.Name }}
app.kubernetes.io/version: {{ .Chart.Version }}
helm.sh/chart: "{{ .Chart.Name }}-{{ .Chart.Version | replace "+" "_" }}"
{{- end -}}

{{- define "amd64_image" -}}
"{{ .Values.aws_account_id }}.dkr.ecr.{{ .Values.aws_default_region }}.amazonaws.com/conesnake:latest-app"
{{- end -}}
