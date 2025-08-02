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

{{- define "amd64_worker_image" -}}
"{{ .Values.gcp_region }}-docker.pkg.dev/{{ .Values.gcp_project }}/conesnake/conesnake:latest-worker-app"
{{- end -}}

{{- define "amd64_relay_image" -}}
"{{ .Values.gcp_region }}-docker.pkg.dev/{{ .Values.gcp_project }}/conesnake/conesnake:latest-relay-app"
{{- end -}}
