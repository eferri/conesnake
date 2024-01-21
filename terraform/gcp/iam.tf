resource "google_project_iam_binding" "conesnake_instance" {
  project = var.project
  role    = google_project_iam_custom_role.conesnake_instance.id
  members = [
    "serviceAccount:${google_service_account.conesnake_instance.email}"
  ]
}

resource "google_project_iam_custom_role" "conesnake_instance" {
  role_id = "${local.deployment}_instance"
  title   = "${local.deployment}_instance"
  permissions = [
    "compute.addresses.use"
  ]
}

resource "google_project_iam_binding" "conesnake_registry" {
  project = var.project
  role    = google_project_iam_custom_role.conesnake_registry.id
  members = [
    "serviceAccount:${google_service_account.conesnake_registry.email}"
  ]
}

resource "google_project_iam_custom_role" "conesnake_registry" {
  role_id = "${local.deployment}_registry"
  title   = "${local.deployment}_registry"
  permissions = [
    "artifactregistry.dockerimages.get",
    "artifactregistry.dockerimages.list",
    "artifactregistry.repositories.downloadArtifacts",
    "artifactregistry.repositories.get",
    "artifactregistry.repositories.list",
    "artifactregistry.repositories.listEffectiveTags",
    "artifactregistry.repositories.listTagBindings",
    "artifactregistry.repositories.uploadArtifacts",
    "artifactregistry.tags.create",
    "artifactregistry.tags.get",
    "artifactregistry.tags.list",
    "artifactregistry.tags.update",
    "artifactregistry.versions.get",
    "artifactregistry.versions.list"
  ]
}
