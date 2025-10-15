# SPDX-License-Identifier: Apache-2.0
# Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

from time import sleep
from edgefirst_client import Client, AnnotationType
from os import environ
from polars import read_ipc
from unittest import TestCase
from tqdm import tqdm
from examples.coco import coco_labels


class BasicTest(TestCase):
    def get_client(self):
        return Client(token=environ.get("STUDIO_TOKEN"))

    def test_version(self):
        client = Client()
        version = client.version()
        assert version != ""

    def test_token(self):
        client = self.get_client()
        token = client.token()
        assert token != ""
        print(f"Token: {token}")
        print(f"Token Expiration: {client.token_expiration}")
        print(f"Username: {client.username}")
        print(f"Server: {client.url}")

        sleep(2)  # Wait for 2 seconds to ensure token renewal updates the time

        client.renew_token()
        new_token = client.token()
        assert new_token != ""
        assert token != new_token
        print(f"New Token Expiration: {client.token_expiration}")

    def test_organization(self):
        client = self.get_client()
        org = client.organization()
        assert org is not None
        assert org.id is not None
        assert org.name is not None
        assert org.credits is not None
        print(f"Organization: {org.name}")
        print(f"ID: {org.id.value}")
        print(f"Credits: {org.credits}")

    def test_projects(self):
        client = self.get_client()
        projects = client.projects()
        assert len(projects) > 0

    def test_project(self):
        client = self.get_client()
        project = client.projects("Unit Testing")
        assert project is not None
        assert len(project) == 1
        assert project[0].name == "Unit Testing"

    def test_datasets(self):
        client = self.get_client()
        project = client.projects("Unit Testing")[0]
        datasets = project.datasets(client)
        assert len(datasets) > 0

        for dataset in datasets:
            ds = client.dataset(dataset.id)
            assert ds is not None
            assert ds.name == dataset.name

            ds = client.datasets(project.id, dataset.name)
            assert len(ds) == 1
            ds = ds[0]
            assert ds.name == dataset.name

    def test_labels(self):
        client = self.get_client()
        projects = client.projects("Unit Testing")
        project_id = projects[0].id
        self.assertIsNotNone(project_id)

        dataset = client.datasets(project_id, "Test Labels")[0]
        for label in dataset.labels(client):
            label.remove(client)

        dataset.add_label(client, "test")
        assert len(dataset.labels(client)) == 1

        dataset.remove_label(client, "test")
        assert len(dataset.labels(client)) == 0

    def test_coco_labels(self):
        client = self.get_client()
        project = client.projects("Sample Project")[0]
        dataset = client.datasets(project.id, "COCO")
        # Filter to avoid fetching the COCO People dataset.
        dataset = filter(lambda d: d.name == "COCO", dataset)
        dataset = next(dataset, None)
        assert dataset is not None
        labels = dataset.labels(client)

        assert len(labels) == 80
        for label in labels:
            assert label.name == coco_labels[label.index]

    def test_annotations_set(self):
        client = self.get_client()
        projects = client.projects("Sample Project")
        project_id = projects[0].id
        self.assertIsNotNone(project_id)

        dataset = client.datasets(project_id, "COCO")
        # Filter to avoid fetching the COCO People dataset.
        dataset = filter(lambda d: d.name == "COCO", dataset)
        dataset = next(dataset, None)
        assert dataset is not None

        with tqdm(total=0, unit="", unit_scale=True, unit_divisor=1000) as bar:

            def progress(current, total):
                if total != bar.total:
                    bar.reset(total)
                bar.update(current - bar.n)

            annotation_set = client.annotation_sets(dataset.id)[0]
            df = client.annotations_dataframe(
                annotation_set_id=annotation_set.id,
                groups=["val"],
                annotation_types=[AnnotationType.Box2d],
                progress=progress,
            )
            df.write_ipc("coco.arrow")
        df = read_ipc("coco.arrow")
        assert len(df.unique("name")) == 5000

    def test_training_sessions(self):
        client = self.get_client()
        project = client.projects("Unit Testing")[0]
        experiment = client.experiments(project.id, "Unit Testing")[0]
        trainers = client.training_sessions(experiment.id)
        assert len(trainers) > 0

        for trainer in trainers:
            t = client.training_session(trainer.id)
            assert t.id.value == trainer.id.value
            assert t.name == trainer.name
            assert t.description == trainer.description
            assert t.dataset_params is not None
            assert t.model_params is not None

        trainer = filter(lambda s: s.name == "modelpack-usermanaged", trainers)
        trainer = next(trainer, None)
        assert trainer is not None

        trainer.set_metrics(client, {"precision": 0.75})
        metrics = trainer.metrics(client)
        assert metrics["precision"] == 0.75

    def test_validation_sessions(self):
        client = self.get_client()
        project = client.projects("Unit Testing")[0]
        sessions = client.validation_sessions(project.id)
        assert len(sessions) > 0

        for session in sessions:
            s = client.validation_session(session.id)
            assert s.id.value == session.id.value
            assert s.name == session.name
            assert s.description == session.description

        session = filter(lambda s: s.name == "modelpack-usermanaged", sessions)
        session = next(session, None)
        assert session is not None

        session.set_metrics(client, {"precision": 0.75})
        metrics = session.metrics(client)
        assert metrics["precision"] == 0.75

    def test_artifacts(self):
        client = self.get_client()
        project = client.projects("Unit Testing")[0]
        experiment = client.experiments(project.id, "Unit Testing")[0]
        trainer = client.training_sessions(experiment.id,
                                           "modelpack-960x540")[0]
        assert trainer.uid.startswith("t-")
        artifacts = client.artifacts(trainer.id)
        assert len(artifacts) > 0

        for artifact in artifacts:
            client.download_artifact(trainer.id, artifact.name)

    def test_checkpoints(self):
        client = self.get_client()
        project = client.projects("Unit Testing")[0]
        experiment = client.experiments(project.id, "Unit Testing")[0]
        trainer = client.training_sessions(experiment.id,
                                           "modelpack-usermanaged")[0]
        assert trainer.uid.startswith("t-")

        with open("checkpoint_py.txt", "w") as f:
            f.write("Checkpoint from Python")

        trainer.upload_checkpoint(client, "checkpoint_py.txt")
        ckpt = trainer.download_checkpoint(client, "checkpoint_py.txt")
        assert ckpt == b"Checkpoint from Python"

    def test_tasks(self):
        client = self.get_client()
        project = client.projects("Unit Testing")[0]
        tasks = client.tasks("modelpack-usermanaged")
        tasks = [client.task_info(t.id) for t in tasks]
        tasks = filter(lambda t: t.project_id.value == project.id.value, tasks)
        task = next(tasks, None)
        assert task is not None

        t = client.task_status(task.id, "export")
        assert t.status == "export"

        client.set_stages(task.id, [("export", "Export Model")])
        client.update_stage(
            task_id=task.id,
            stage="export",
            status="running",
            message="Exporting the model",
            percentage=25,
        )

        task = client.task_info(task.id)
        assert len(task.stages) == 1
        assert task.stages["export"].message == "Exporting the model"
        assert task.stages["export"].status == "running"
        assert task.stages["export"].percentage == 25
