from __future__ import annotations

from datetime import datetime
from enum import Enum
from pathlib import Path
from polars import DataFrame
from typing import Any, Callable, Dict, Optional, List, Tuple, Union

#: Progress callback for long-running operations.
#:
#: This type represents a callback function that receives progress information
#: for operations like file uploads, downloads, or dataset processing. The
#: callback receives the current count and total count to enable progress
#: reporting in applications.
#:
#: Examples:
#:     >>> def progress_callback(current: int, total: int) -> None:
#:     ...     percentage = (current / total) * 100.0
#:     ...     print(f"Progress: {percentage:.1f}% ({current}/{total})")
#:     >>>
#:     >>> client.upload_dataset("/path/to/data", "ds-abc123",
#:     ...                      progress=progress_callback)
Progress = Callable[[int, int], None]


class Parameter:
    """
    Represents a parameter value that can be an integer, float, boolean,
    string, array, or object (dictionary).

    This class provides Python magic methods for type conversions and
    comparisons, making it behave like native Python types.

    Examples:
        >>> # Create Parameters using static constructors
        >>> p_int = Parameter.integer(42)
        >>> p_real = Parameter.real(3.14)
        >>> p_bool = Parameter.boolean(True)
        >>> p_str = Parameter.string("hello")
        >>> p_array = Parameter.array([1, 2, 3])
        >>> p_obj = Parameter.object({"key": "value"})
        >>>
        >>> # Type conversions
        >>> int(p_int)  # Returns: 42
        >>> float(p_int)  # Returns: 42.0
        >>> float(p_real)  # Returns: 3.14
        >>> int(p_real)  # Returns: 3
        >>>
        >>> # Type checking
        >>> p_real.is_real()  # Returns: True
        >>> p_real.type_name()  # Returns: "Real"
        >>>
        >>> # Equality with tolerance for floats
        >>> p_real = Parameter.real(0.75)
        >>> p_real == 0.75  # Returns: True (with tolerance)
        >>> p_real == 0.75000000001  # Returns: True (within epsilon)
    """

    @staticmethod
    def integer(value: int) -> "Parameter":
        """Create an Integer parameter."""
        ...

    @staticmethod
    def real(value: float) -> "Parameter":
        """Create a Real (float) parameter."""
        ...

    @staticmethod
    def boolean(value: bool) -> "Parameter":
        """Create a Boolean parameter."""
        ...

    @staticmethod
    def string(value: str) -> "Parameter":
        """Create a String parameter."""
        ...

    @staticmethod
    def array(values: List[Any]) -> "Parameter":
        """
        Create an Array parameter from a Python list.

        Values in the list will be converted to Parameters recursively.
        """
        ...

    @staticmethod
    def object(values: Dict[str, Any]) -> "Parameter":
        """
        Create an Object parameter from a Python dictionary.

        Values in the dict will be converted to Parameters recursively.
        """
        ...

    def is_integer(self) -> bool:
        """Check if this is an Integer parameter."""
        ...

    def is_real(self) -> bool:
        """Check if this is a Real parameter."""
        ...

    def is_boolean(self) -> bool:
        """Check if this is a Boolean parameter."""
        ...

    def is_string(self) -> bool:
        """Check if this is a String parameter."""
        ...

    def is_array(self) -> bool:
        """Check if this is an Array parameter."""
        ...

    def is_object(self) -> bool:
        """Check if this is an Object parameter."""
        ...

    def type_name(self) -> str:
        """
        Get the variant type name.

        Returns one of: "Integer", "Real", "Boolean", "String",
        "Array", "Object"
        """
        ...

    def variant_type(self) -> str:
        """
        Get the variant type name.

        Returns one of: "Integer", "Real", "Boolean", "String",
        "Array", "Object"

        This is an alias for type_name() for consistency with Rust API.
        """
        ...

    def as_integer(self) -> Optional[int]:
        """
        Extract as Python int if this is an Integer parameter.

        Returns None if this is not an Integer parameter.

        Examples:
            >>> p = Parameter.integer(42)
            >>> p.as_integer()  # Returns: 42
            >>> Parameter.real(3.14).as_integer()  # Returns: None
        """
        ...

    def as_real(self) -> Optional[float]:
        """
        Extract as Python float if this is a Real parameter.

        Returns None if this is not a Real parameter.

        Examples:
            >>> p = Parameter.real(3.14)
            >>> p.as_real()  # Returns: 3.14
            >>> Parameter.integer(42).as_real()  # Returns: None
        """
        ...

    def as_boolean(self) -> Optional[bool]:
        """
        Extract as Python bool if this is a Boolean parameter.

        Returns None if this is not a Boolean parameter.

        Examples:
            >>> p = Parameter.boolean(True)
            >>> p.as_boolean()  # Returns: True
            >>> Parameter.integer(1).as_boolean()  # Returns: None
        """
        ...

    def as_string(self) -> Optional[str]:
        """
        Extract as Python str if this is a String parameter.

        Returns None if this is not a String parameter.

        Examples:
            >>> p = Parameter.string("hello")
            >>> p.as_string()  # Returns: "hello"
            >>> Parameter.integer(42).as_string()  # Returns: None
        """
        ...

    def as_array(self) -> Optional[List[Any]]:
        """
        Extract as Python list if this is an Array parameter.

        Returns None if this is not an Array parameter (caller should check type).
        Elements are converted to native Python types recursively.

        Examples:
            >>> p = Parameter.array([
            ...     Parameter.integer(1),
            ...     Parameter.real(2.5),
            ...     Parameter.string("test")
            ... ])
            >>> p.as_array()  # Returns: [1, 2.5, "test"]
            >>> Parameter.integer(42).as_array()  # Returns: None (not an Array)
        """
        ...

    def as_object(self) -> Optional[Dict[str, Any]]:
        """
        Extract as Python dict if this is an Object parameter.

        Returns None if this is not an Object parameter (caller should check type).
        Values are converted to native Python types recursively.

        Examples:
            >>> p = Parameter.object({
            ...     "count": Parameter.integer(42),
            ...     "ratio": Parameter.real(3.14)
            ... })
            >>> p.as_object()  # Returns: {"count": 42, "ratio": 3.14}
            >>> Parameter.integer(42).as_object()  # Returns: None (not an Object)
        """
        ...

    def __int__(self) -> int:
        """Convert to Python int (works for Integer, Real, Boolean)."""
        ...

    def __float__(self) -> float:
        """Convert to Python float (works for Integer, Real, Boolean)."""
        ...

    def __bool__(self) -> bool:
        """Convert to Python bool (works for all types)."""
        ...

    def __str__(self) -> str:
        """
        Convert to Python string.

        For String parameters, returns the plain string value without
        decoration.
        For other types, returns a descriptive representation.

        Examples:
            >>> str(Parameter.string("hello"))  # Returns: "hello"
            >>> str(Parameter.integer(42))      # Returns: "Integer(42)"
        """
        ...

    def __repr__(self) -> str:
        """
        Python representation (always descriptive).

        Examples:
            >>> repr(Parameter.string("hello"))  # Returns: "String(hello)"
            >>> repr(Parameter.integer(42))      # Returns: "Integer(42)"
        """
        ...

    def __eq__(self, other: Any) -> bool:
        """
        Equality comparison with type coercion.

        For numeric types (Integer, Real), compares with tolerance
        (epsilon=1e-9).
        """
        ...

    def get(self, key: str, default: Any = None) -> Any:
        """
        Get value by key with optional default (Object only).

        Works like dict.get() - returns the value if key exists,
        otherwise returns the default value.

        Args:
            key: The key to lookup
            default: Value to return if key doesn't exist (default: None)

        Returns:
            The value converted to native Python types, or default if
            key not found

        Raises:
            TypeError: If parameter is not an Object

        Examples:
            >>> obj = Parameter.object({"model": Parameter.string("yolo")})
            >>> obj.get("model")  # Returns: "yolo"
            >>> obj.get("missing")  # Returns: None
            >>> obj.get("missing", "default")  # Returns: "default"
        """
        ...

    def keys(self) -> List[str]:
        """
        Get dictionary keys (Object only).

        Returns:
            List of keys in the object

        Raises:
            TypeError: If parameter is not an Object

        Examples:
            >>> obj = Parameter.object({"a": 1, "b": 2})
            >>> obj.keys()  # Returns: ["a", "b"]
        """
        ...

    def values(self) -> List[Any]:
        """
        Get dictionary values (Object only).

        Returns:
            List of values converted to native Python types

        Raises:
            TypeError: If parameter is not an Object

        Examples:
            >>> obj = Parameter.object({
            ...     "a": Parameter.integer(1),
            ...     "b": Parameter.integer(2)
            ... })
            >>> obj.values()  # Returns: [1, 2]
        """
        ...

    def items(self) -> List[Tuple[str, Any]]:
        """
        Get dictionary items as (key, value) tuples (Object only).

        Returns:
            List of (key, value) tuples with values converted to
            native Python types

        Raises:
            TypeError: If parameter is not an Object

        Examples:
            >>> obj = Parameter.object({
            ...     "a": Parameter.integer(1),
            ...     "b": Parameter.integer(2)
            ... })
            >>> obj.items()  # Returns: [("a", 1), ("b", 2)]
        """
        ...


#: Type alias for parameter values used in metrics and configurations
ParameterValue = Union[str, float, bool]
ParameterDict = Dict[
    str,
    Union[
        Parameter,
        ParameterValue,
        List[ParameterValue],
        Dict[str, ParameterValue],
    ],
]


class Error(Exception):
    ...


class ProjectID:
    """
    Unique identifier for a project within EdgeFirst Studio.

    Projects contain datasets, experiments, and models within an organization.
    Each project has a unique ID displayed in hexadecimal format with a "p-"
    prefix (e.g., "p-def456").

    Examples:
        >>> project_id = ProjectID.from_str("p-def456")
        >>> print(project_id.value)  # Returns: 14644310
        >>> print(str(project_id))   # Returns: "p-def456"

    Note:
        Internally a ProjectID is an unsigned 64-bit integer. This class
        handles translating between integer and string representations of
        project IDs.
    """

    @property
    def value(self) -> int:
        """
        Returns the integer value of the project ID.
        """
        ...


class DatasetID:
    """
    Unique identifier for a dataset within a project.

    Datasets contain collections of images, annotations, and other data used
    for machine learning experiments. Each dataset has a unique ID displayed
    in hexadecimal format with a "ds-" prefix (e.g., "ds-123abc").

    Examples:
        >>> dataset_id = DatasetID.from_str("ds-456def")
        >>> print(dataset_id.value)  # Returns: 4508143
        >>> print(str(dataset_id))   # Returns: "ds-456def"

    Note:
        Internally a DatasetID is an unsigned 64-bit integer. This class
        handles translating between integer and string representations of
        dataset IDs.
    """

    @property
    def value(self) -> int:
        """
        Returns the integer value of the dataset ID.
        """
        ...


class ExperimentID:
    """
    Unique identifier for an experiment within a project.

    Experiments represent individual machine learning experiments with specific
    configurations, datasets, and results. Each experiment has a unique ID
    displayed in hexadecimal format with an "exp-" prefix (e.g., "exp-123abc").

    Examples:
        >>> exp_id = ExperimentID.from_str("exp-456def")
        >>> print(exp_id.value)  # Returns: 4508143
        >>> print(str(exp_id))   # Returns: "exp-456def"

    Note:
        Internally an ExperimentID is an unsigned 64-bit integer. This class
        handles translating between integer and string representations of
        experiment IDs.
    """

    @property
    def value(self) -> int:
        """
        Returns the integer value of the experiment ID.
        """
        ...


class OrganizationID:
    """
    Unique identifier for an organization in EdgeFirst Studio.

    Organizations are the top-level containers for users, projects, and
    resources in EdgeFirst Studio. Each organization has a unique ID that is
    displayed in hexadecimal format with an "org-" prefix (e.g., "org-abc123").

    Examples:
        >>> org_id = OrganizationID.from_str("org-abc123")
        >>> print(org_id.value)  # Returns: 11256099
        >>> print(str(org_id))   # Returns: "org-abc123"

    Note:
        Internally an OrganizationID is an unsigned 64-bit integer. This class
        handles translating between integer and string representations of
        organization IDs.
    """

    @property
    def value(self) -> int:
        """
        Returns the integer value of the organization ID.
        """
        ...


class SampleID:
    """
    Unique identifier for a sample in EdgeFirst Studio. Internally a
    SampleID is an unsigned 64-bit integer. Samples are represented with
    a type identifier followed by the ID in hex, for example 'smp-6c5b4a'.
    This class handles translating between integer and string
    representations of sample IDs.
    """

    @property
    def value(self) -> int:
        """
        Returns the integer value of the sample ID.
        """
        ...


class AnnotationSetID:
    """
    Unique identifier for an annotation set in EdgeFirst Studio. Internally
    an AnnotationSetID is an unsigned 64-bit integer. Annotation sets are
    represented with a type identifier followed by the ID in hex, for
    example 'as-3d2c1b'. This class handles translating between integer
    and string representations of annotation set IDs.
    """

    @property
    def value(self) -> int:
        """
        Returns the integer value of the annotation set ID.
        """
        ...


class TaskID:
    """
    Unique identifier for a task in EdgeFirst Studio. Internally a TaskID
    is an unsigned 64-bit integer. Tasks are represented with a type
    identifier followed by the ID in hex, for example 'tsk-8e7d6c'. This
    class handles translating between integer and string representations of
    task IDs.
    """

    @property
    def value(self) -> int:
        """
        Returns the integer value of the task ID.
        """
        ...


class TrainingSessionID:
    """
    Unique identifier for a training session within an experiment.

    Training sessions represent individual training runs with specific
    hyperparameters and configurations. Each training session has a unique ID
    displayed in hexadecimal format with a "t-" prefix (e.g., "t-789012").

    Examples:
        >>> training_id = TrainingSessionID.from_str("t-abc123")
        >>> print(training_id.value)  # Returns: 11256099
        >>> print(str(training_id))   # Returns: "t-abc123"

    Note:
        Internally a TrainingSessionID is an unsigned 64-bit integer. This
        class handles translating between integer and string representations
        of training session IDs.
    """

    @property
    def value(self) -> int:
        """
        Returns the integer value of the training session ID.
        """
        ...


class ValidationSessionID:
    """
    Unique identifier for a validation session within an experiment.

    Validation sessions represent model validation runs that evaluate trained
    models against test datasets. Each validation session has a unique ID
    displayed in hexadecimal format with a "v-" prefix (e.g., "v-345678").

    Examples:
        >>> validation_id = ValidationSessionID.from_str("v-deadbeef")
        >>> print(validation_id.value)  # Returns: 3735928559
        >>> print(str(validation_id))   # Returns: "v-deadbeef"

    Note:
        Internally a ValidationSessionID is an unsigned 64-bit integer. This
        class handles translating between integer and string representations
        of validation session IDs.
    """

    @property
    def value(self) -> int:
        """
        Returns the integer value of the validation session ID.
        """
        ...


class SnapshotID:
    """
    Unique identifier for a snapshot in EdgeFirst Studio.

    Snapshots represent saved states of datasets or model checkpoints.
    Each snapshot has a unique ID displayed in hexadecimal format with
    a "ss-" prefix (e.g., "ss-f1e2d3").

    Examples:
        >>> snapshot_id = SnapshotID.from_str("ss-abc123")
        >>> print(snapshot_id.value)  # Returns: 11256099
        >>> print(str(snapshot_id))   # Returns: "ss-abc123"

    Note:
        Internally a SnapshotID is an unsigned 64-bit integer. This class
        handles translating between integer and string representations of
        snapshot IDs.
    """

    @property
    def value(self) -> int:
        """
        Returns the integer value of the snapshot ID.
        """
        ...


class ImageId:
    """
    Unique identifier for an image in EdgeFirst Studio. Internally an
    ImageId is an unsigned 64-bit integer. Images are represented with a
    type identifier followed by the ID in hex, for example 'img-4c3b2a'.
    This class handles translating between integer and string
    representations of image IDs.
    """

    @property
    def value(self) -> int:
        """
        Returns the integer value of the image ID.
        """
        ...


class SequenceId:
    """
    Unique identifier for a sequence in EdgeFirst Studio. Internally a
    SequenceId is an unsigned 64-bit integer. Sequences are represented
    with a type identifier followed by the ID in hex, for example
    'seq-7f6e5d'. This class handles translating between integer and string
    representations of sequence IDs.
    """

    @property
    def value(self) -> int:
        """
        Returns the integer value of the sequence ID.
        """
        ...


class AppId:
    """
    Unique identifier for an application in EdgeFirst Studio. Internally
    an AppId is an unsigned 64-bit integer. Applications are represented
    with a type identifier followed by the ID in hex, for example
    'app-2e1d0c'. This class handles translating between integer and string
    representations of application IDs.
    """

    @property
    def value(self) -> int:
        """
        Returns the integer value of the application ID.
        """
        ...


# Type aliases for User ID patterns (TypeUID = Type User ID)
ProjectUID = Union[ProjectID, int, str]
DatasetUID = Union[DatasetID, int, str]
ExperimentUID = Union[ExperimentID, int, str]
OrganizationUID = Union[OrganizationID, int, str]
SampleUID = Union[SampleID, int, str]
AnnotationSetUID = Union[AnnotationSetID, int, str]
TaskUID = Union[TaskID, int, str]
TrainingSessionUID = Union[TrainingSessionID, int, str]
ValidationSessionUID = Union[ValidationSessionID, int, str]
SnapshotUID = Union[SnapshotID, int, str]
ImageUID = Union[ImageId, int, str]
SequenceUID = Union[SequenceId, int, str]
AppUID = Union[AppId, int, str]


class Organization:
    """
    Organization information and metadata.

    Each user belongs to an organization which contains projects, datasets,
    and other resources. Organizations provide isolated workspaces for teams
    and manage resource quotas and billing.

    Examples:
        >>> # Access organization details
        >>> org = client.organization()
        >>> print(f"Organization: {org.name} (ID: {org.id})")
        >>> print(f"Available credits: {org.credits}")
    """
    @property
    def id(self) -> OrganizationID:
        """
        The unique identifier for the organization.
        """
        ...

    @property
    def name(self) -> str:
        """
        The name of the organization.
        """
        ...

    @property
    def credits(self) -> int:
        """
        The number of credits available to the organization.
        """
        ...


class Project:
    """
    The project class represents a project in the EdgeFirst Studio.  A project
    contains datasets, experiments, and other resources related to a specific
    task or workflow.
    """

    @property
    def id(self) -> ProjectID:
        """
        The unique identifier for the project.
        """
        ...

    @property
    def uid(self) -> str:
        """
        The unique string identifier for the object.

        .. deprecated::
            Use ``str(Project.id)`` instead. This property will be removed
            in a future version.
        """
        ...

    @property
    def name(self) -> str:
        """
        The name of the project.
        """
        ...

    @property
    def description(self) -> str:
        """
        The description of the project.
        """
        ...

    def datasets(
        self, client: Client, name: Optional[str] = None
    ) -> List[Dataset]:
        """
        List the datasets in the project.

        Args:
            client: The client to use for the request.
            name: The name of the dataset to filter by.

        Returns:
            A list of datasets in the project.
        """
        ...


class AnnotationSet:
    """
    The AnnotationSet class represents the collection of annotations for a
    given dataset.  A dataset can have multiple annotation sets, each
    containing annotations for different tasks or purposes.
    """

    @property
    def id(self) -> AnnotationSetID:
        """The ID of the annotation set."""
        ...

    @property
    def uid(self) -> str:
        """
        The unique string identifier for the object.

        .. deprecated::
            Use ``str(AnnotationSet.id)`` instead. This property will be
            removed in a future version.
        """
        ...

    @property
    def dataset_id(self) -> DatasetID:
        """The ID of the dataset that the annotation set belongs to."""
        ...

    @property
    def name(self) -> str:
        """The name of the annotation set."""
        ...

    @property
    def description(self) -> str:
        """The description of the annotation set."""
        ...

    @property
    def created(self) -> datetime:
        """The creation date of the annotation set."""
        ...


class Label:
    """
    Representation of a label in EdgeFirst Studio.  Labels are used to identify
    annotations in a dataset.
    """

    @property
    def id(self) -> int:
        """The ID of the label."""
        ...

    @property
    def uid(self) -> str:
        """
        The unique string identifier for the object.

        .. deprecated::
            Use ``str(Label.id)`` instead. This property will be removed
            in a future version.
        """
        ...

    @property
    def name(self) -> str:
        """The name of the label."""
        ...

    @property
    def index(self) -> int:
        """The index of the label."""
        ...

    @property
    def dataset_id(self) -> DatasetID:
        """The ID of the dataset that the label belongs to."""
        ...

    def remove(self, client: Client) -> None:
        """Remove the label from the dataset."""
        ...

    def set_name(self, client: Client, name: str) -> None:
        """Set the name of the label."""
        ...

    def set_index(self, client: Client, index: int) -> None:
        """Set the index of the label."""
        ...


class Dataset:
    """
    A dataset in EdgeFirst Studio containing sensor data and annotations.

    Datasets are collections of multi-modal sensor data (images, LiDAR, radar)
    along with their corresponding annotations (bounding boxes, segmentation
    masks, 3D annotations). Datasets belong to projects and can be used for
    training and validation of machine learning models.

    Features:
        - **Multi-modal Data**: Support for images, LiDAR point clouds, radar
          data
        - **Rich Annotations**: 2D/3D bounding boxes, segmentation masks
        - **Metadata**: Timestamps, sensor configurations, calibration data
        - **Version Control**: Track changes and maintain data lineage
        - **Format Conversion**: Export to popular ML frameworks

    Examples:
        Basic dataset operations:

        >>> # Get dataset information
        >>> dataset = client.dataset("ds-abc123")
        >>> print(f"Dataset: {dataset.name}")
        >>> print(f"Created: {dataset.created}")
        >>> print(f"Description: {dataset.description}")

        >>> # Access dataset contents through client
        >>> samples = client.samples(dataset.id)
        >>> annotations = client.annotation_sets(dataset.id)

        >>> # Manage dataset labels
        >>> dataset.add_label(client, "training")
        >>> labels = dataset.labels(client)

    Note:
        The dataset class represents a dataset in EdgeFirst Studio. A dataset
        is a collection of sensor data such as images, lidar, radar along with
        annotations for bounding boxes, masks, or 3d bounding boxes.
    """

    @property
    def id(self) -> DatasetID:
        """The ID of the dataset."""
        ...

    @property
    def uid(self) -> str:
        """
        The unique string identifier for the object.

        .. deprecated::
            Use ``str(Dataset.id)`` instead. This property will be removed
            in a future version.
        """
        ...

    @property
    def name(self) -> str:
        """The name of the dataset."""
        ...

    @property
    def description(self) -> str:
        """The description of the dataset."""
        ...

    @property
    def created(self) -> datetime:
        """The creation date of the dataset."""
        ...

    def labels(self, client: Client) -> List[Label]:
        """The labels associated with the dataset."""
        ...

    def add_label(self, client: Client, name: str) -> None:
        """Add a label to the dataset."""
        ...

    def remove_label(self, client: Client, name: str) -> None:
        """Remove a label from the dataset."""
        ...


class FileType(Enum):
    """
    File types supported in EdgeFirst Studio datasets.

    Represents the different types of sensor data files that can be stored
    and processed in a dataset. EdgeFirst Studio supports various modalities
    including visual images and different forms of LiDAR and radar data.

    Examples:
        >>> # Create file types from strings
        >>> image_type = FileType.Image
        >>> lidar_type = FileType.LidarPcd

        >>> # Use in dataset operations
        >>> files_by_type = dataset.get_files_by_type(FileType.Image)

    Members:
        Image:
            Standard image files (JPEG, PNG, etc.). A file with extension
            `.image.jpeg` that stores the image.
        LidarPcd:
            LiDAR point cloud data files (.pcd format). A file with extension
            `.lidar.pcd` that stores [x, y, z] Cartesian coordinates from the
            LiDAR sensor.
        LidarDepth:
            LiDAR depth images (.png format). A file with extension
            `.lidar.png` that stores per-pixel depth values captured by the
            LiDAR sensor.
        LidarReflect:
            LiDAR reflectance images (.jpg format). A file with extension
            `.lidar.jpeg` that stores reflectance data from the LiDAR sensor.
        RadarPcd:
            Radar point cloud data files (.pcd format). A file with extension
            `.radar.pcd` that stores [x, y, z] Cartesian coordinates in meters
            from the Radar sensor, along with speed m/s, power, noise, and
            radar cross-section (RCS).
        RadarCube:
            Radar cube data files (.png format). A file with extension
            `.radar.png` that stores a range-doppler radar cube. The cube has
            dimensions: sequence, rx_antenna, range_bins, doppler_bins —
            encoded in a 16-bit PNG.
    """

    Image: "FileType"
    LidarPcd: "FileType"
    LidarDepth: "FileType"
    LidarReflect: "FileType"
    RadarPcd: "FileType"
    RadarCube: "FileType"


class AnnotationType(Enum):
    """
    Annotation types supported for labeling data in EdgeFirst Studio.

    Represents the different types of annotations that can be applied to
    sensor data for machine learning tasks. Each type corresponds to a
    different annotation geometry and use case.

    Examples:
        >>> # Create annotation types
        >>> box_2d = AnnotationType.Box2d
        >>> segmentation = AnnotationType.Mask

        >>> # Use in dataset queries
        >>> annotations = dataset.get_annotations_by_type(AnnotationType.Box2d)

    Members:
        Box2d: 2D bounding boxes for object detection in images
        Box3d: 3D bounding boxes for object detection in 3D space (LiDAR, etc.)
        Mask:  Pixel-level segmentation masks for semantic/instance
               segmentation
    """

    Box2d: "AnnotationType"
    Box3d: "AnnotationType"
    Mask: "AnnotationType"


class Box2d:
    """
    The Box2d is a representation of a single 2D bounding box annotation
    which is a pixel-based annotation for the image that is stored in
    EdgeFirst Datasets.  The bounding boxes are normalized to the image
    dimensions (float32 values between 0 and 1).  The width and height of the
    box are provided through the `width` and `height` properties while the
    position of the box can be represented in two ways: either through the
    `left` and `top` properties which represent the top-left corner of
    the box or through the `cx` and `cy` properties which represent the
    center of the box.
    """

    def __init__(
        self, left: float, top: float, width: float, height: float
    ) -> None:
        """
        Create a new bounding box representation given the coordinates
        [xc, yc, width, height] or [xmin, ymin, width, height] of the
        bounding box.  The coordinates should be normalized to
        the image dimensions.

        Args:
            x (float): The normalized x-center or xmin coordinate
                       of the bounding box.
            y (float): The normalized y-center or ymin coordinate
                       of the bounding box.
            width (float): The normalized width of the bounding box.
            height (float): The normalized height of the bounding box.
        """
        ...

    @property
    def width(self) -> float:
        """
        Returns the width of the bounding box. This dimension is
        normalized to the image width.

        Returns:
            float: The width of the bounding box.
        """
        ...

    @property
    def height(self) -> float:
        """
        Returns the height of the bounding box.  This dimension is normalized
        to the image height.

        Returns:
            float: The height of the bounding box.
        """
        ...

    @property
    def left(self) -> float:
        """
        Returns the left coordinate of the bounding box.  This is either
        the x-center or xmin coordinate of the bounding box.

        Returns:
            float: The left coordinate of the bounding box.
        """
        ...

    @property
    def top(self) -> float:
        """
        Returns the y-coordinate of the bounding box.  This is either
        the y-center or ymin coordinate of the bounding box.

        Returns:
            float: The y-coordinate of the bounding box.
        """
        ...

    @property
    def cx(self) -> float:
        """
        Returns the x-center coordinate of the bounding box.  This coordinate
        is normalized to the image width.

        Returns:
            float: The x-center coordinate of the bounding box.
        """
        ...

    @property
    def cy(self) -> float:
        """
        Returns the y-center coordinate of the bounding box.  This coordinate
        is normalized to the image height.

        Returns:
            float: The y-center coordinate of the bounding box.
        """
        ...


class Box3d:
    """
    The Box3d is a representation of a single 3D bounding box annotation
    which is based in meters.  The bounding boxes are float32 values containing
    the fields [x, y, z, width, height, depth].  This follows the convention
    for the Sensor Coordinate Frame where the x-axis is forward, y-axis is
    left, and z-axis is up.
    """

    def __init__(
        self,
        cx: float,
        cy: float,
        cz: float,
        width: float,
        height: float,
        depth: float,
    ) -> None:
        """
        Initialize a 3D bounding box with the given position and dimensions.

        Args:
            cx (float): The x-coordinate of the box center (forward).
            cy (float): The y-coordinate of the box center (left).
            cz (float): The z-coordinate of the box center (up).
            width (float): The width of the box along the y-axis.
            height (float): The height of the box along the z-axis.
            depth (float): The depth of the box along the x-axis.
        """
        ...

    @property
    def width(self) -> float:
        """
        The width of the bounding box along the y-axis.

        Returns:
            float: The width in meters.
        """
        ...

    @property
    def height(self) -> float:
        """
        The height of the bounding box along the z-axis.

        Returns:
            float: The height in meters.
        """
        ...

    @property
    def length(self) -> float:
        """
        The length of the bounding box along the x-axis.

        Returns:
            float: The length in meters.
        """
        ...

    @property
    def cx(self) -> float:
        """
        The x-coordinate of the box center (forward direction).

        Returns:
            float: The x-coordinate in meters.
        """
        ...

    @property
    def cy(self) -> float:
        """
        The y-coordinate of the box center (left direction).

        Returns:
            float: The y-coordinate in meters.
        """
        ...

    @property
    def cz(self) -> float:
        """
        The z-coordinate of the box center (up direction).

        Returns:
            float: The z-coordinate in meters.
        """
        ...

    @property
    def left(self) -> float:
        """
        The left coordinate of the bounding box along the y-axis.

        Returns:
            float: The left coordinate in meters.
        """
        ...

    @property
    def top(self) -> float:
        """
        The top coordinate of the bounding box along the z-axis.

        Returns:
            float: The top coordinate in meters.
        """
        ...

    @property
    def front(self) -> float:
        """
        The front coordinate of the bounding box along the x-axis.

        Returns:
            float: The front coordinate in meters.
        """
        ...


class Mask:
    """
    Represents a segmentation mask using polygonal annotations.

    The mask is defined by one or more polygons, where each polygon is
    a list of [x, y] coordinates normalized to the image dimensions.
    All coordinates are float32 values between 0 and 1.
    """

    def __init__(self, polygon: List[List[float]]) -> None:
        """
        Initializes a new Mask instance from a list of polygons.

        Args:
            polygon (List[List[float]]): A list of polygons, where each polygon
                                         is a list of [x, y] float coordinates
                                         normalized to the image dimensions.
        """
        ...

    @property
    def polygon(self) -> List[List[float]]:
        """
        Returns the polygon data defining the mask.

        Each polygon is a list of [x, y] coordinates, with values
        normalized to the image dimensions.

        Returns:
            List[List[float]]: A list of polygons representing the mask.
        """
        ...


class SampleFile:
    """
    Represents a file associated with a sample (e.g., LiDAR, radar, depth map).

    For uploading samples, create a SampleFile with a type and local filename.
    For downloaded samples, the file will have a type and URL.
    """

    def __init__(self, file_type: str, filename: str) -> None:
        """
        Create a new sample file for upload.

        Args:
            file_type: Type of file (e.g., "image", "lidar", "depth")
            filename: Path to the local file to upload
        """
        ...

    @property
    def file_type(self) -> str:
        """The type of file (e.g., "image", "lidar")."""
        ...

    @property
    def filename(self) -> Optional[str]:
        """Local filename for upload, or None if downloaded."""
        ...

    @property
    def url(self) -> Optional[str]:
        """URL for downloaded files, or None if for upload."""
        ...


class PresignedUrl:
    """
    A presigned URL for uploading a file to S3.

    Returned by populate_samples to indicate where files should be uploaded.
    """

    @property
    def filename(self) -> str:
        """The filename as specified in the sample."""
        ...

    @property
    def key(self) -> str:
        """The S3 key path."""
        ...

    @property
    def url(self) -> str:
        """The presigned URL for uploading (use PUT request)."""
        ...


class SamplesCountResult:
    """
    Result of counting samples in a dataset.

    Contains the total number of samples matching the specified criteria.
    """

    @property
    def total(self) -> int:
        """The total number of samples."""
        ...


class SamplesPopulateResult:
    """
    Result of populating a sample into a dataset.

    Contains the UUID assigned to the sample and presigned URLs
    for uploading associated files.
    """

    @property
    def uuid(self) -> str:
        """The UUID assigned to the sample by the server."""
        ...

    @property
    def urls(self) -> List[PresignedUrl]:
        """Presigned URLs for uploading files associated with this sample."""
        ...


class Annotation:
    """
    Represents a single annotation associated
    with a sample in an EdgeFirst dataset.

    An annotation may include sensor metadata such as
    (name, group, label, etc.) as well as 2D/3D bounding boxes
    and segmentation masks.
    """

    def __init__(self) -> None:
        """Create a new empty annotation."""
        ...

    def set_label(self, label: Optional[str]) -> None:
        """Set the label for this annotation."""
        ...

    def set_object_id(self, object_id: Optional[str]) -> None:
        """Set the object identifier for this annotation."""
        ...

    def set_object_reference(self, object_reference: Optional[str]) -> None:
        """Legacy alias for :meth:`set_object_id`."""
        ...

    def set_box2d(self, box2d: Optional[Box2d]) -> None:
        """Set the 2D bounding box for this annotation."""
        ...

    def set_box3d(self, box3d: Optional[Box3d]) -> None:
        """Set the 3D bounding box for this annotation."""
        ...

    def set_mask(self, mask: Optional[Mask]) -> None:
        """Set the segmentation mask for this annotation."""
        ...

    @property
    def sample_id(self) -> Optional[SampleID]:
        """
        The ID of the sample this annotation belongs to.

        Returns:
            Optional[SampleID]: The sample ID, or None if not available.
        """
        ...

    @property
    def name(self) -> Optional[str]:
        """
        The name of the annotation instance, if specified.  The name
        is derived from device hostname and the date and time and the
        specific frame from the recording "hostname_date_time_frame".

        Returns:
            Optional[str]: The instance name or None.
        """
        ...

    @property
    def group(self) -> Optional[str]:
        """
        The group this annotation belongs to, if specified.  A group
        can be "train", "val", etc.

        Returns:
            Optional[str]: The group name or None.
        """
        ...

    @property
    def sequence_name(self) -> Optional[str]:
        """
        The sequence name this annotation is part of, if any.  The sequence
        name is derived from the device hostname and the date and time
        of the recording hostname_date_time.

        Returns:
            Optional[str]: The sequence name or None.
        """
        ...

    @property
    def object_id(self) -> Optional[str]:
        """
        A unique identifier for the object associated with this annotation.

        Returns:
            Optional[str]: The object identifier or None.
        """
        ...

    @property
    def object_reference(self) -> Optional[str]:
        """Legacy alias for :attr:`object_id`."""
        ...

    @property
    def label(self) -> Optional[str]:
        """
        The semantic label (e.g., "car", "pedestrian") for this annotation.

        Returns:
            Optional[str]: The label or None.
        """
        ...

    @property
    def label_index(self) -> Optional[int]:
        """
        The index of the label for this annotation.

        Returns:
            Optional[int]: The label index or None.
        """
        ...

    @property
    def box2d(self) -> Optional[Box2d]:
        """
        The 2D bounding box associated with this annotation, if available.

        Returns:
            Optional[Box2d]: The 2D bounding box or None.
        """
        ...

    @property
    def box3d(self) -> Optional[Box3d]:
        """
        The 3D bounding box associated with this annotation, if available.

        Returns:
            Optional[Box3d]: The 3D bounding box or None.
        """
        ...

    @property
    def mask(self) -> Optional[Mask]:
        """
        The segmentation mask associated with this annotation, if available.

        Returns:
            Optional[Mask]: The segmentation mask or None.
        """
        ...


class Sample:
    """
    Represents a single data sample in the EdgeFirst dataset.
    A sample includes metadata and associated annotations, and
    can be used to download file content for different sensor modalities.
    """

    def __init__(self) -> None:
        """Create a new empty sample."""
        ...

    def set_image_name(self, image_name: Optional[str]) -> None:
        """Set the image filename for this sample."""
        ...

    def set_group(self, group: Optional[str]) -> None:
        """Set the dataset split (train/val/test) for this sample."""
        ...

    def set_sequence_name(self, sequence_name: Optional[str]) -> None:
        """Set the sequence name for this sample."""
        ...

    def set_sequence_uuid(self, sequence_uuid: Optional[str]) -> None:
        """Set the unique sequence identifier for this sample."""
        ...

    def set_sequence_description(
        self, sequence_description: Optional[str]
    ) -> None:
        """Set the sequence description for this sample."""
        ...

    def set_frame_number(self, frame_number: Optional[int]) -> None:
        """Set the frame number for this sample."""
        ...

    def add_file(self, file: SampleFile) -> None:
        """Add a file (image, LiDAR, etc.) to this sample."""
        ...

    def add_annotation(self, annotation: Annotation) -> None:
        """Add an annotation to this sample."""
        ...

    @property
    def id(self) -> Optional[SampleID]:
        """
        Returns the unique identifier of the sample.

        Returns:
            Optional[SampleID]: The sample ID, or None if not set.
        """
        ...

    @property
    def uid(self) -> Optional[str]:
        """
        The unique string identifier for the object.

        .. deprecated::
            Use ``str(Sample.id)`` if id is not None. This property will be
            removed in a future version.

        Returns:
            Optional[str]: The UID, or None if not set.
        """
        ...

    @property
    def name(self) -> Optional[str]:
        """
        Returns the name of the sample.

        Returns:
            Optional[str]: The sample name, or None if not set.
        """
        ...

    @property
    def group(self) -> Optional[str]:
        """
        Returns the group name of the sample, if any.

        Returns:
            Optional[str]: The group name or None.
        """
        ...

    @property
    def sequence_name(self) -> Optional[str]:
        """
        Returns the sequence name to which this sample belongs, if any.

        Returns:
            Optional[str]: The sequence name or None.
        """
        ...

    @property
    def sequence_uuid(self) -> Optional[str]:
        """
        Returns the UUID of the sequence to which this sample belongs, if any.

        Returns:
            Optional[str]: The sequence UUID or None.
        """
        ...

    @property
    def sequence_description(self) -> Optional[str]:
        """
        Returns the description of the sequence to which this sample
        belongs, if any.

        Returns:
            Optional[str]: The sequence description or None.
        """
        ...

    @property
    def frame_number(self) -> Optional[int]:
        """
        Returns the frame number of this sample within its sequence, if any.

        Returns:
            Optional[int]: The frame number or None.
        """
        ...

    @property
    def uuid(self) -> Optional[str]:
        """
        Returns the UUID of this sample.

        Returns:
            Optional[str]: The sample UUID or None.
        """
        ...

    @property
    def image_name(self) -> Optional[str]:
        """
        Returns the image filename for this sample.

        Returns:
            Optional[str]: The image filename or None.
        """
        ...

    @property
    def image_url(self) -> Optional[str]:
        """
        Returns the URL of the image for this sample.

        Returns:
            Optional[str]: The image URL or None.
        """
        ...

    @property
    def width(self) -> Optional[int]:
        """
        Returns the width of the image in pixels.

        Returns:
            Optional[int]: The image width or None.
        """
        ...

    @property
    def height(self) -> Optional[int]:
        """
        Returns the height of the image in pixels.

        Returns:
            Optional[int]: The image height or None.
        """
        ...

    @property
    def date(self) -> Optional[str]:
        """
        Returns the timestamp when this sample was captured, in RFC3339 format.

        Returns:
            Optional[str]: The timestamp as an ISO 8601 string or None.
        """
        ...

    @property
    def source(self) -> Optional[str]:
        """
        Returns the source identifier for this sample.

        Returns:
            Optional[str]: The source identifier or None.
        """
        ...

    @property
    def files(self) -> List[SampleFile]:
        """
        Returns the list of files associated with this sample.

        Returns:
            List[SampleFile]: A list of sample file objects.
        """
        ...

    @property
    def annotations(self) -> List[Annotation]:
        """
        Returns the list of annotations associated with this sample.

        Returns:
            List[Annotation]: A list of annotation objects.
        """
        ...

    def download(
        self, client: Client, file_type: FileType = FileType.Image
    ) -> Optional[bytes]:
        """
        Downloads the data file for this sample using the given file type.

        Args:
            client (Client): The client instance used to download the file.
            file_type (FileType, optional): The type of file to download.
                                            Defaults to FileType.Image.

        Returns:
            Optional[bytes]: The raw file data as bytes,
                             or None if no file exists.
        """
        ...


class Experiment:
    """
    Represents an experiment in EdgeFirst Studio which are used to organize
    training and validation sessions.
    """

    @property
    def id(self) -> ExperimentID:
        """
        Returns the unique identifier of the experiment.

        Returns:
            ID: The experiment ID.
        """
        ...

    @property
    def uid(self) -> str:
        """
        The unique string identifier for the object.

        .. deprecated::
            Use ``str(Experiment.id)`` instead. This property will be
            removed in a future version.
        """
        ...

    @property
    def name(self) -> str:
        """
        Returns the name of the experiment.

        Returns:
            str: The experiment name.
        """
        ...

    @property
    def description(self) -> Optional[str]:
        """
        Returns the description of the experiment, if any.

        Returns:
            Optional[str]: The experiment description or None.
        """
        ...


class Task:
    """
    Represents an EdgeFirst Studio Cloud Task.  A task could be a docker
    instance or an EC2 instance or similar.
    """

    @property
    def id(self) -> TaskID:
        """
        Returns the unique identifier of the Docker task.

        Returns:
            TaskID: The Docker task ID.
        """
        ...

    @property
    def uid(self) -> str:
        """
        The unique string identifier for the object.

        .. deprecated::
            Use ``str(Task.id)`` instead. This property will be removed
            in a future version.
        """
        ...

    @property
    def name(self) -> str:
        """
        Returns the name of the Docker task.

        Returns:
            str: The Docker task name.
        """
        ...

    @property
    def status(self) -> str:
        """
        Returns the status of the Docker task.

        Returns:
            str: The Docker task status.
        """
        ...

    @property
    def workflow(self) -> str:
        """
        Returns the task workflow which could be trainer or validation
        workflows.

        Returns:
            str: The task workflow.
        """
        ...

    @property
    def manager(self) -> str:
        """
        Returns the manager type for the task.  The manager could be cloud,
        user, or kubernetes.

        Returns:
            str: The task manager type.
        """
        ...

    @property
    def instance(self) -> str:
        """
        Returns the instance type for the task.  The instance type depends on
        the manager, for cloud manager it is the AWS EC2 instance type.

        Returns:
            str: The task instance type.
        """
        ...

    @property
    def created(self) -> datetime:
        """
        Returns the creation date of the Docker task.

        Returns:
            datetime: The Docker task creation date.
        """
        ...


class Stage:
    """
    Represents a stage in the task.
    """

    @property
    def task_id(self) -> TaskID:
        """
        Returns the ID of the task associated with this stage.

        Returns:
            ID: The task ID.
        """
        ...

    @property
    def stage_id(self) -> str:
        """
        Returns the ID of the stage.

        Returns:
            str: The stage ID.
        """
        ...

    @property
    def status(self) -> Optional[str]:
        """
        Returns the status of the stage.

        Returns:
            str: The stage status.
        """
        ...

    @property
    def description(self) -> Optional[str]:
        """
        Returns the description of the stage, if any.

        Returns:
            Optional[str]: The stage description or None.
        """
        ...

    @property
    def message(self) -> Optional[str]:
        """
        Returns the message associated with the stage, if any.

        Returns:
            Optional[str]: The stage message or None.
        """
        ...

    @property
    def percentage(self) -> int:
        """
        Returns the completion percentage of the stage.

        Returns:
            int: The stage completion percentage as an integer, each step
                 representing 1% of the total.
        """
        ...


class TaskInfo:
    """
    The TaskInfo class provides detailed information about a specific task such
    as its status and progress.
    """

    @property
    def id(self) -> TaskID:
        """
        Returns the unique identifier of the task.

        Returns:
            TaskID: The task ID.
        """
        ...

    @property
    def uid(self) -> str:
        """
        Returns the unique string identifier for the task.

        .. deprecated::
            Use ``str(TaskInfo.id)`` instead. This property will be
            removed in a future version.

        Returns:
            str: The task UID.
        """
        ...

    @property
    def project_id(self) -> ProjectID:
        """
        Returns the ID of the project associated with the task.

        Returns:
            ProjectID: The project ID.
        """
        ...

    @property
    def description(self) -> str:
        """
        Returns the description of the task, if any.

        Returns:
            Optional[str]: The task description or None.
        """
        ...

    @property
    def status(self) -> str:
        """
        Returns the status of the task.

        Returns:
            str: The task status.
        """
        ...

    @property
    def stages(self) -> Dict[str, Stage]:
        """
        Returns the stages of the task.

        Returns:
            Dict[str, Stage]: A dictionary of stage names to Stage objects.
        """
        ...

    @property
    def created(self) -> datetime:
        """
        Returns the creation date of the task.

        Returns:
            datetime: The task creation date.
        """
        ...

    @property
    def completed(self) -> datetime:
        """
        Returns the completion date of the task, if any.

        Returns:
            datetime: The task completion date or None.
        """
        ...

    def set_status(self, client: Client, status: str) -> None:
        """
        Sets the status of the task.

        Args:
            client (Client): The EdgeFirst client.
            status (str): The new status for the task.
        """
        ...

    def set_stages(
        self, client: Client, stages: List[Tuple[str, str]]
    ) -> None:
        """
        Sets the stages of the task.

        Args:
            client (Client): The EdgeFirst client.
            stages (List[Tuple[str, str]]): A list of tuples containing stage
                                            names and descriptions.
        """

    def update_stage(
        self,
        client: Client,
        stage_name: str,
        status: Optional[str] = None,
        message: Optional[str] = None,
        percentage: Optional[int] = None,
    ) -> None:
        """
        Updates a specific stage of the task.

        Args:
            client (Client): The EdgeFirst client.
            stage_name (str): The name of the stage to update.
            status (Optional[str]): The new status for the stage.
            message (Optional[str]): A message associated with the stage.
            percentage (Optional[int]): The completion percentage of the stage.
        """
        ...


class DatasetParams:
    """
    Represents the parameters for a dataset used in a training session.
    """

    @property
    def dataset_id(self) -> DatasetID:
        """
        Returns the ID of the dataset associated with these parameters.

        Returns:
            DatasetID: The dataset ID.
        """
        ...

    @property
    def annotation_set_id(self) -> AnnotationSetID:
        """
        Returns the ID of the annotation set associated with these parameters.

        Returns:
            AnnotationSetID: The annotation set ID.
        """
        ...

    @property
    def train_group(self) -> str:
        """
        Returns the name of the selected training group.

        Returns:
            str: The training group name.
        """
        ...

    @property
    def val_group(self) -> str:
        """
        Returns the name of the selected validation group.

        Returns:
            str: The validation group name.
        """
        ...


class Artifact:
    """
    Represents an artifact produced by a training session.
    """

    @property
    def name(self) -> str:
        """
        Returns the name of the artifact.

        Returns:
            str: The artifact name.
        """
        ...

    @property
    def model_type(self) -> str:
        """
        Returns the type of the model used in the artifact.

        Returns:
            str: The model type.
        """
        ...


class TrainingSession:
    """
    A training session for a specific experiment, this represents the
    configuration and state of the training process.
    """

    @property
    def id(self) -> TrainingSessionID:
        """
        Returns the unique identifier of the training session.

        Returns:
            TrainingSessionID: The training session ID.
        """
        ...

    @property
    def uid(self) -> str:
        """
        The unique string identifier for the object.

        .. deprecated::
            Use ``str(TrainingSession.id)`` instead. This property will
            be removed in a future version.
        """
        ...

    @property
    def experiment_id(self) -> ExperimentID:
        """
        Returns the ID of the experiment associated with this training session.

        Returns:
            ExperimentID: The experiment ID.
        """
        ...

    @property
    def name(self) -> str:
        """
        Returns the name of the training session.

        Returns:
            str: The training session name.
        """
        ...

    @property
    def description(self) -> Optional[str]:
        """
        Returns the description of the training session, if any.

        Returns:
            Optional[str]: The training session description or None.
        """
        ...

    @property
    def model(self) -> str:
        """
        Returns the model used in the training session.

        Returns:
            str: The model implementation name.
        """
        ...

    @property
    def task(self) -> Task:
        """
        Returns the Docker task associated with the training session.

        Returns:
            Task: The Docker task object.
        """
        ...

    @property
    def model_params(self) -> Dict[str, Parameter]:
        """
        Returns the model parameters used in the training session.

        Returns:
            Dict[str, Parameter]: The model parameters.
        """
        ...

    @property
    def dataset_params(self) -> DatasetParams:
        """
        Returns the dataset parameters used in the training session.

        Returns:
            DatasetParams: The dataset parameters object.
        """
        ...

    def metrics(self, client: Client) -> Dict[str, Parameter]:
        """
        Returns the metrics associated with the training session.

        Args:
            client (Client): The EdgeFirst client.

        Returns:
            Dict[str, Parameter]: The training session metrics.
        """
        ...

    def set_metrics(
        self, client: Client, metrics: Dict[str, Parameter]
    ) -> None:
        """
        Sets the metrics for the training session.

        Args:
            client (Client): The EdgeFirst client.
            metrics (Dict[str, Parameter]): The training session metrics.
        """
        ...

    def artifacts(self, client: Client) -> List[Artifact]:
        """
        Returns a list of artifacts produced by the training session.

        Args:
            client (Client): The EdgeFirst client.

        Returns:
            List[Artifact]: A list of artifacts.
        """
        ...

    def upload_artifact(
        self, client: Client, filename: str, path: Optional[Path] = None
    ) -> None:
        """
        Uploads an artifact file to the training session.

        Args:
            client (Client): The EdgeFirst client.
            filename (str): The name of the artifact file.
            path (Optional[Path]): The local path to the artifact file.  If not
                                   specified the filename is used as the path.
        """
        ...

    def download_artifact(self, client: Client, filename: str) -> bytes:
        """
        Downloads the specified artifact file from the training session.  Note
        that to download with progress to an output file you can use the
        `client.download_artifact` method instead.

        Args:
            client (Client): The EdgeFirst client.
            filename (str): The name of the artifact file to download.

        Returns:
            bytes: The raw file data as bytes.
        """
        ...

    def upload_checkpoint(
        self, client: Client, filename: str, path: Optional[Path] = None
    ) -> None:
        """
        Uploads a checkpoint file to the training session.

        Args:
            client (Client): The EdgeFirst client.
            filename (str): The name of the checkpoint file.
            path (Optional[Path]): The local path to the checkpoint file. If
                                   not specified the filename is used as the
                                   path.
        """
        ...

    def download_checkpoint(self, client: Client, filename: str) -> bytes:
        """
        Downloads the specified checkpoint file from the training session.
        Note that to download with progress to an output file you can use the
        `client.download_artifact` method instead.

        Args:
            client (Client): The EdgeFirst client.
            filename (str): The filename for the checkpoint file to download.

        Returns:
            bytes: The raw file data as bytes.
        """
        ...

    def upload(self, client: Client, files: List[Tuple[str, Path]]) -> None:
        """
        Uploads files to the training session.  This can be used to upload
        model weights or other files that are needed for the training session.

        The first element in the files tuple is the target name for the file
        while the second element is the local path to the file.  The target
        name is the path where the file will be stored in the training session.

        Artifacts must be uploaded to `artifacts/*`, checkpoints to
        `checkpoints/*`,  while metrics should be uploaded to `metrics/*`.

        Args:
            client (Client): The EdgeFirst client.
            files (List[Tuple[str, Path]]): A list of tuples containing the
                                            target filename and the path to the
                                            file to upload.
        """
        ...

    def download(self, client: Client, filename: str) -> str:
        """
        Downloads the specified file from the training session.  This function
        requires the target file to only contain valid utf-8 as it is returned
        through a JSON response.  To retrieve binary files use the
        `client.download_artifact` method instead.

        Args:
            client (Client): The EdgeFirst client.
            filename (str): The name of the file to download.

        Returns:
            str: The raw file data as a string.
        """
        ...


class ValidationSession:
    """
    This class represents a validation session for a given model and dataset.
    """

    @property
    def id(self) -> ValidationSessionID:
        """
        Returns the unique identifier of the validation session.

        Returns:
            ValidationSessionID: The validation session ID.
        """
        ...

    @property
    def uid(self) -> str:
        """
        Returns the unique string identifier of the validation session.

        .. deprecated::
            Use ``str(ValidationSession.id)`` instead. This property will
            be removed in a future version.

        Returns:
            str: The validation session UID.
        """
        ...

    @property
    def name(self) -> str:
        """
        Returns the name of the validation session.

        Returns:
            str: The validation session name.
        """
        ...

    @property
    def description(self) -> str:
        """
        Returns the description of the validation session, if any.

        Returns:
            str: The validation session description or an empty string.
        """
        ...

    @property
    def dataset_id(self) -> DatasetID:
        """
        Returns the ID of the dataset associated with this validation session.

        Returns:
            DatasetID: The dataset ID.
        """
        ...

    @property
    def experiment_id(self) -> ExperimentID:
        """
        Returns the ID of the experiment associated with this validation
        session.

        Returns:
            ExperimentID: The experiment ID.
        """
        ...

    @property
    def training_session_id(self) -> TrainingSessionID:
        """
        Returns the ID of the training session associated with this validation
        session.

        Returns:
            TrainingSessionID: The training session ID.
        """
        ...

    @property
    def annotation_set_id(self) -> AnnotationSetID:
        """
        Returns the ID of the annotation set associated with this validation
        session.

        Returns:
            AnnotationSetID: The annotation set ID.
        """
        ...

    @property
    def params(self) -> Dict[str, Parameter]:
        """
        Returns the parameters associated with this validation session.

        Returns:
            Dict[str, Parameter]: The validation session parameters.
        """
        ...

    @property
    def task(self) -> Task:
        """
        Returns the Docker task associated with the validation session.

        Returns:
            Task: The Docker task object.
        """
        ...

    def metrics(self, client: Client) -> Dict[str, Parameter]:
        """
        Returns the metrics associated with the validation session.

        Args:
            client (Client): The EdgeFirst client.

        Returns:
            Dict[str, Parameter]: The validation session metrics.
        """
        ...

    def set_metrics(self, client: Client, metrics: Dict[str, Parameter]):
        """
        Sets the metrics for the validation session.

        Args:
            client (Client): The EdgeFirst client.
            metrics (Dict[str, Parameter]): The metrics to set.
        """
        ...

    def artifacts(self, client: Client) -> List[Artifact]:
        """
        Returns a list of artifacts produced by the validation session.

        Args:
            client (Client): The EdgeFirst client.

        Returns:
            List[Artifact]: A list of artifacts.
        """
        ...

    def upload(self, client: Client, files: List[Tuple[str, Path]]):
        """
        Uploads the specified files to the validation session.

        Args:
            client (Client): The EdgeFirst client.
            files (List[Tuple[str, Path]]): A list of tuples containing the
                                              target filename and the path to
                                              the file to upload.
        """
        ...


class Client:
    """
    Main client for interacting with EdgeFirst Studio Server.

    The EdgeFirst Client handles the connection to the EdgeFirst Studio Server
    and manages authentication, RPC calls, and data operations. It provides
    methods for managing projects, datasets, experiments, training sessions,
    and various utility functions for data processing.

    The client supports multiple authentication methods and can work with both
    SaaS and self-hosted EdgeFirst Studio instances.

    Features:
        - **Authentication**: Token-based authentication with automatic
          persistence
        - **Dataset Management**: Upload, download, and manipulate datasets
        - **Project Operations**: Create and manage projects and experiments
        - **Training & Validation**: Submit and monitor ML training jobs
        - **Data Integration**: Convert between EdgeFirst datasets and popular
          formats
        - **Progress Tracking**: Real-time progress updates for long-running
          operations

    Examples:
        Basic client setup and authentication:

        >>> # Create a new client and authenticate
        >>> client = Client()
        >>> client.login("your-email@example.com", "password")

        >>> # Or use an existing token
        >>> client = Client(token="your-token-here")

        >>> # Get organization and list projects
        >>> org = client.organization()
        >>> projects = client.projects()  # Returns all projects
        >>> filtered = client.projects("project-name")  # Filter by name

        >>> # Work with datasets
        >>> dataset = client.dataset("ds-abc123")
        >>> datasets = client.datasets()  # List all datasets

    Note:
        The client also provides various utility methods for interacting with
        datasets and converting them to and from Polars DataFrames.
    """

    def __init__(
        self,
        token: Optional[str] = None,
        username: Optional[str] = None,
        password: Optional[str] = None,
        server: Optional[str] = None,
        use_token_file: bool = True,
    ) -> None:
        """
        Create a new EdgeFirst client instance.  The client has a few options
        for authentication.  A token can be provided directly to the client
        which is used to authenticate the client with the server and includes
        the server instance information.  Alternatively, a username and
        password can be provided to the client along with an optional server
        otherwise the default of "saas" is used.  If none of these options are
        provided the client will use the local user's configuration file to
        read the last saved token unless the `use_token_file` parameter is set
        to false.

        Args:
            token (Optional[str]): The authentication token for the client.
                                   If provided, this is used to authenticate
                                   the client with the server.
            username (Optional[str]): The username to log in to Studio.
            password (Optional[str]): The password to log in to Studio.
            server (Optional[str]): The server to connect to.  If not provided,
                                    the default server "saas" is used.
            use_token_file (bool): Whether to use the local token file for
                                   authentication if no token, username, or
                                   password is provided.  Defaults to true.
        """
        ...

    def version(self) -> str:
        """
        Return the version of the EdgeFirst Studio server for the current
        client connection.

        Returns:
            str: The version of the server.
        """
        ...

    def token(self) -> str:
        """
        Return the token used to authenticate the client with the server.  When
        logging into the server using a username and password, the token is
        returned by the server and stored in the client for future
        interactions.

        Returns:
            str: The token used to authenticate the client with the server.
        """
        ...

    def verify_token(self):
        """
        Verify the token used to authenticate the client with the server.  This
        method is used to ensure that the token is still valid and has not
        expired.  If the token is invalid, the server will return an error and
        the client will need to login again.

        Raises:
            Error: If the token is invalid or expired.
        """
        ...

    def renew_token(self):
        """
        Renew the token used to authenticate the client with the server.  This
        method is used to extend the expiration time of the token.  If the
        token is invalid or expired, the server will return an error and the
        client will need to login again.

        Raises:
            Error: If the token is invalid or expired.
        """
        ...

    @property
    def token_expiration(self) -> datetime:
        """
        Return the expiration date of the token used to authenticate the
        client with the server.

        Returns:
            datetime: The expiration date of the token.
        """
        ...

    def login(self, username: str, password: str):
        """
        Login to the server using the specified username and password.  The
        server will authenticate the user and return a token that can be used
        to authenticate future requests.  The token is stored in the client and
        used to authenticate the client with the server.

        Args:
            username (str): The username to log in to EdgeFirst Studio.
            password (str): The password to log in to EdgeFirst Studio.

        Raises:
            Error: If authentication fails.
        """
        ...

    @property
    def username(self) -> str:
        """
        Return the username associated with the current client.

        Returns:
            str: The username associated with the current client.

        Raises:
            Error: If the username cannot be accessed.
        """
        ...

    @property
    def url(self) -> str:
        """
        Return the server URL associated with the current client.

        Returns:
            str: The server URL associated with the current client.
        """
        ...

    def organization(self) -> Organization:
        """
        Return the organization associated with the current user.  The
        organization is the top-level entity in EdgeFirst Studio and contains
        projects, datasets, trainers, and trainer sessions.

        Returns:
            Organization: The organization associated with the current user.

        Raises:
            Error: If the organization cannot be accessed.
        """
        ...

    def projects(self, name: Optional[str] = None) -> List[Project]:
        """
        Returns a list of projects available to the user.  The projects are
        returned as a vector of Project objects.  If the name parameter is
        provided, only projects containing the specified name will be returned.

        Projects are the top-level organizational unit in EdgeFirst Studio.
        Projects contain datasets, trainers, and trainer sessions.  Projects
        are used to group related datasets and trainers together.

        Args:
            name (Optional[str]): The name of the project to filter by.

        Returns:
            List[Project]: A list of accessible projects.
        """
        ...

    def project(self, project_id: ProjectUID) -> Project:
        """
        Return the project with the specified project ID.  If the project does
        not exist, an error is returned.

        Args:
            project_id (ProjectUID): The ID of the project to retrieve.

        Returns:
            Project: The requested project.

        Raises:
            Error: If the project does not exist or cannot be accessed.
        """
        ...

    def datasets(
        self, project_id: ProjectUID, name: Optional[str] = None
    ) -> List[Dataset]:
        """
        Returns a list of datasets available to the user.  The datasets are
        returned as a vector of Dataset objects.  If a name is provided, only
        datasets with that name are returned.

        Args:
            project_id (ProjectUID): The project ID whose datasets to get.
            name (Optional[str]): The name of the dataset to filter by.

        Returns:
            List[Dataset]: A list of datasets.
        """
        ...

    def dataset(self, dataset_id: DatasetUID) -> Dataset:
        """
        Return the dataset with the specified dataset ID.  If the dataset does
        not exist, an error is returned.

        Args:
            dataset_id (DatasetUID): The ID of the dataset.

        Returns:
            Dataset: The requested dataset.

        Raises:
            Error: If the dataset does not exist or cannot be accessed.
        """
        ...

    def labels(self, dataset_id: DatasetUID) -> List[Label]:
        """Get the labels associated with the dataset."""
        ...

    def add_label(self, dataset_id: DatasetUID, name: str) -> None:
        """Add a label to the dataset."""
        ...

    def remove_label(self, label_id: int) -> None:
        """Remove a label from the dataset."""
        ...

    def update_label(self, label: Label) -> None:
        """
        Update the properties of a label.

        Args:
            label (Label): The label to update with modified properties.

        Raises:
            Error: If the label does not exist or cannot be updated.
        """
        ...

    def create_dataset(
        self, project_id: str, name: str, description: Optional[str] = None
    ) -> str:
        """
        Create a new dataset in the specified project.

        Args:
            project_id (str): ID of the project to create the dataset in.
            name (str): Name of the new dataset.
            description (Optional[str]): Optional description for the
                dataset. Defaults to None.

        Returns:
            str: Dataset ID of the newly created dataset.
        """
        ...

    def delete_dataset(self, dataset_id: DatasetUID) -> None:
        """
        Delete a dataset by marking it as deleted.

        Args:
            dataset_id (Union[DatasetID, int, str]): ID of the dataset
                to delete.
        """
        ...

    def create_annotation_set(
        self,
        dataset_id: DatasetUID,
        name: str,
        description: Optional[str] = None,
    ) -> str:
        """
        Create a new annotation set for the specified dataset.

        Args:
            dataset_id (Union[DatasetID, int, str]): ID of the dataset to
                create the annotation set in.
            name (str): Name of the new annotation set.
            description (Optional[str]): Optional description for the
                annotation set. Defaults to None.

        Returns:
            str: Annotation set ID of the newly created annotation set.
        """
        ...

    def delete_annotation_set(
        self, annotation_set_id: AnnotationSetUID
    ) -> None:
        """
        Delete an annotation set by marking it as deleted.

        Args:
            annotation_set_id (Union[AnnotationSetID, int, str]): ID of the
                annotation set to delete.
        """
        ...

    def download_dataset(
        self,
        dataset_id: DatasetUID,
        groups: List[str] = [],
        types: List[FileType] = [],
        output: Optional[str] = None,
        progress: Optional[Progress] = None,
    ):
        """
        Download dataset samples matching specified groups and file types.

        Args:
            dataset_id (Union[DatasetID, int, str]): ID of the dataset.
            groups (List[str]): Dataset groups to include (train, val, etc).
            types (List[FileType]): File types to download.
            output (str): Output directory to save downloaded files.
            progress (Optional[Progress]): Optional progress reporter.
        """
        ...

    def annotation_sets(self, dataset_id: DatasetUID) -> List[AnnotationSet]:
        """
        Retrieve the annotation sets associated with the specified dataset.

        Args:
            dataset_id (Union[DatasetID, int, str]): Dataset ID.

        Returns:
            List[AnnotationSet]: Annotation sets associated with the dataset.
        """
        ...

    def annotation_set(
        self, annotation_set_id: AnnotationSetUID
    ) -> AnnotationSet:
        """
        Retrieve the annotation set with the specified ID.

        Args:
            annotation_set_id (AnnotationSetUID): Annotation set ID.

        Returns:
            AnnotationSet: The requested annotation set.
        """
        ...

    def annotations(
        self,
        annotation_set_id: AnnotationSetUID,
        groups: List[str] = [],
        annotation_types: List[AnnotationType] = [],
        progress: Optional[Progress] = None,
    ) -> List[Annotation]:
        """
        Get the annotations for the specified annotation set with the
        requested annotation types.  The annotation types are used to filter
        the annotations returned.  The groups parameter is used to filter for
        dataset groups (train, val, test).  Images which do not have any
        annotations are also included in the result as long as they are in the
        requested groups (when specified).

        The result is a vector of Annotations objects which contain the
        full dataset along with the annotations for the specified types.

        To get the annotations as a DataFrame, use the `annotations_dataframe`
        method instead.

        Args:
            annotation_set_id (AnnotationSetUID): The ID of the annotation set.
            groups (List[str]): Dataset groups to include.
            annotation_types (List[AnnotationType]): Types of annotations
                                                     to include.
            progress (Optional[Progress]): Optional progress reporter.

        Returns:
            List[Annotation]: List of annotations.
        """
        ...

    def annotations_dataframe(
        self,
        annotation_set_id: AnnotationSetUID,
        groups: List[str] = [],
        annotation_types: List[AnnotationType] = [AnnotationType.Box2d],
        progress: Optional[Progress] = None,
    ) -> DataFrame:
        """
        Get the AnnotationGroup for the specified annotation set with the
        requested annotation types.  The annotation type is used to filter
        the annotations returned.  Images which do not have any annotations
        are included in the result.

        The result is a DataFrame following the EdgeFirst Dataset Format
        definition.

        To get the annotations as a vector of AnnotationGroup objects, use the
        `annotations` method instead.

        .. deprecated::
            Use ``samples_dataframe()`` for complete 2025.10 schema support.
            This method will be removed in a future version.

        Args:
            annotation_set_id (AnnotationSetUID): ID of the annotation set.
            groups (List[str]): Dataset groups to include.
            annotation_types (List[AnnotationType]): Types of annotations to
                                                     include.
            progress (Optional[Sender[Progress]]): Optional progress channel.

        Returns:
            DataFrame: A Polars DataFrame containing the annotations.
        """
        ...

    def samples_dataframe(
        self,
        dataset_id: DatasetUID,
        annotation_set_id: Optional[AnnotationSetUID] = None,
        groups: List[str] = [],
        annotation_types: List[AnnotationType] = [],
        progress: Optional[Progress] = None,
    ) -> DataFrame:
        """
        Get samples as a DataFrame with complete 2025.10 schema.

        This method returns a DataFrame with 13 columns following the
        EdgeFirst Dataset Format 2025.10 specification, including optional
        metadata columns (size, location, pose, degradation) that may be
        populated from sample sensor data.

        Args:
            dataset_id (Union[DatasetID, int, str]): ID of the dataset.
            annotation_set_id (AnnotationSetUID): Optional annotation set
                                                filter.
            groups (List[str]): Dataset groups to include.
            annotation_types (List[AnnotationType]): Types of annotations to
                                                     include.
            progress (Optional[Sender[Progress]]): Optional progress channel.

        Returns:
            DataFrame: A Polars DataFrame with 13 columns (name, frame,
                      object_id, label, label_index, group, mask, box2d,
                      box3d, size, location, pose, degradation).
        """
        ...

    def samples_count(
        self,
        dataset_id: DatasetUID,
        annotation_set_id: Optional[AnnotationSetUID] = None,
        annotation_types: List[AnnotationType] = [],
        groups: List[str] = [],
        types: List[FileType] = [FileType.Image],
    ) -> SamplesCountResult:
        """
        Count samples in a dataset without fetching them.

        This method returns only the total count of samples matching the
        specified criteria, which is much faster than fetching all samples
        when you only need to know how many exist.

        Args:
            dataset_id (Union[DatasetID, int, str]): ID of the dataset.
            annotation_set_id (AnnotationSetUID): The ID of the annotation
                                                set to filter by.
            annotation_types (List[AnnotationType]): Types of annotations
                                                        to include.
            groups (List[str]): Dataset groups to include.
            types (List[FileType]): Type of files to include.

        Returns:
            SamplesCountResult: Object with total count of matching samples.
        """
        ...

    def samples(
        self,
        dataset_id: DatasetUID,
        annotation_set_id: Optional[AnnotationSetUID] = None,
        annotation_types: List[AnnotationType] = [AnnotationType.Box2d],
        groups: List[str] = [],
        types: List[FileType] = [FileType.Image],
        progress: Optional[Progress] = None,
    ) -> List[Sample]:
        """
        Retrieve sample metadata and annotations for a dataset.

        Args:
            dataset_id (Union[DatasetID, int, str]): ID of the dataset.
            annotation_set_id (AnnotationSetUID): The ID of the annotation
                                                set to fetch.
            annotation_types (List[AnnotationType]): Types of annotations
                                                        to include.
            groups (List[str]): Dataset groups to include.
            types (List[FileType]): Type of files to include.
            progress (Optional[Progress]): Optional progress reporter.

        Returns:
            List[Sample]: A list of sample objects.
        """
        ...

    def populate_samples(
        self,
        dataset_id: DatasetUID,
        annotation_set_id: AnnotationSetUID,
        samples: List[Sample],
        progress: Optional[Progress] = None,
    ) -> List[SamplesPopulateResult]:
        """
        Populate samples into a dataset with automatic file uploads.

        This method creates new samples in the specified dataset and
        automatically uploads their associated files (images, LiDAR, etc.)
        to S3 using presigned URLs.

        The server will auto-generate UUIDs and extract image dimensions
        for samples that don't have them specified.

        Args:
            dataset_id: ID of the dataset to populate
            annotation_set_id: ID of the annotation set for sample
                              annotations
            samples: List of Sample objects to create (with files
                    and annotations)
            progress: Optional callback function(current, total)
                     for upload progress

        Returns:
            List[SamplesPopulateResult]: List of results with UUIDs
                                        and presigned URLs

        Example:
            >>> from edgefirst_client import (
            ...     Client, Sample, SampleFile, Annotation, Box2d
            ... )
            >>> client = Client()
            >>> sample = Sample()
            >>> sample.set_image_name("test.png")
            >>> sample.add_file(SampleFile("image", "path/to/test.png"))
            >>> annotation = Annotation()
            >>> annotation.set_label("car")
            >>> annotation.set_box2d(Box2d(10.0, 20.0, 100.0, 50.0))
            >>> sample.add_annotation(annotation)
            >>> results = client.populate_samples(
            ...     dataset_id,
            ...     annotation_set_id,
            ...     [sample],
            ...     lambda curr, total: print(f"{curr}/{total}")
            ... )
        """
        ...

    def experiments(
        self, project_id: ProjectUID, name: Optional[str] = None
    ) -> List[Experiment]:
        """
        Returns a list of experiments available to the user.  The experiments
        are returned as a vector of Experiment objects.  Experiments provide a
        method of organizing training and validation sessions together and are
        akin to an Experiment in MLFlow terminology.  Each experiment can have
        multiple trainer sessions associated with it, these would be akin to
        runs in MLFlow terminology.

        Args:
            project_id (ProjectUID): The project ID for which to list
                experiments.
            name (Optional[str]): The name of the experiment to filter by.

        Returns:
            List[Experiment]: A list of Experiment objects
                              associated with the project.

        Raises:
            Error: If the server request fails.
        """
        ...

    def experiment(self, experiment_id: ExperimentUID) -> Experiment:
        """
        Return the experiment with the specified experiment ID.  If the
        experiment does not exist, an error is returned.

        Args:
            experiment_id (ExperimentUID): The ID of the experiment to fetch.

        Returns:
            Experiment: The Experiment object corresponding to the given ID.

        Raises:
            Error: If the experiment does not exist or request fails.
        """
        ...

    def training_sessions(
        self, trainer_id: ExperimentUID, name: Optional[str] = None
    ) -> List[TrainingSession]:
        """
        Returns a list of trainer sessions available to the user.  The trainer
        sessions are returned as a vector of TrainingSession objects.  Trainer
        sessions are akin to runs in MLFlow terminology.  These represent an
        actual training session which will produce metrics and model artifacts.

        Args:
            trainer_id (ExperimentUID): The ID of the trainer/experiment.
            name (Optional[str]): The name of the trainer session to filter by.

        Returns:
            List[TrainingSession]: A list of trainer sessions
                                  under the experiment.

        Raises:
            Error: If the request fails.
        """
        ...

    def training_session(
        self, session_id: TrainingSessionUID
    ) -> TrainingSession:
        """
        Return the training session with the specified training session ID.  If
        the training session does not exist, an error is returned.

        Args:
            session_id (TrainingSessionUID): The training session ID.

        Returns:
            TrainingSession: The training session with the specified ID.

        Raises:
            Error: If the session does not exist or the request fails.
        """
        ...

    def validation_sessions(
        self, project_id: ProjectUID
    ) -> List[ValidationSession]:
        """
        Returns a list of validation sessions associated with the specified
        project.

        Args:
            project_id (ProjectUID): The project ID to retrieve validation
                sessions for.

        Returns:
            List[ValidationSession]: A list of validation session objects.

        Raises:
            Error: If the request fails.
        """
        ...

    def validation_session(
        self, session_id: ValidationSessionUID
    ) -> ValidationSession:
        """Return the validation session with the specified ID.

        Args:
            session_id (ValidationSessionUID): The validation session ID.

        Returns:
            ValidationSession: The validation session with the specified ID.

        Raises:
            Error: If the validation session does not exist or the request
                   fails.
        """
        ...

    def artifacts(self, session_id: TrainingSessionUID) -> List[Artifact]:
        """
        List the artifacts for the specified training session.  The artifacts
        are returned as a vector of strings.

        Args:
            session_id (TrainingSessionUID): The training session ID.

        Returns:
            List[Artifact]: A list of artifact objects
                            generated by the session.

        Raises:
            Error: If the request fails.
        """
        ...

    def download_artifact(
        self,
        training_session_id: TrainingSessionUID,
        modelname: str,
        filename: Optional[Path] = None,
        progress: Optional[Progress] = None,
    ) -> None:
        """
        Download the model artifact for the specified trainer session to the
        specified file path.  A progress callback can be provided to monitor
        the progress of the download over a watch channel.

        Args:
            training_session_id (TrainingSessionUID): ID of the trainer
                session the model belongs to.
            modelname (str): Name of the model file to download.
            filename (Optional[Path]): Local file path to save the downloaded
                                       artifact.  If not specified, the
                                       modelname is used as the filename.
            progress (Optional[Progress]): Optional progress callback.

        Raises:
            Error: If the download fails or cannot be written to disk.
        """
        ...

    def download_checkpoint(
        self,
        training_session_id: TrainingSessionUID,
        checkpoint: str,
        filename: Optional[Path] = None,
        progress: Optional[Progress] = None,
    ) -> None:
        """
        Download the checkpoint file for the specified trainer session to the
        specified file path.  A progress callback can be provided to monitor
        the progress of the download over a watch channel.

        Args:
            training_session_id (TrainingSessionUID): ID of the trainer
                session the checkpoint belongs to.
            checkpoint (str): Name of the checkpoint file to download.
            filename (Optional[Path]): Local file path to save the downloaded
                                       checkpoint.  If not specified, the
                                       checkpoint name is used as the filename.
            progress (Optional[Progress]): Optional progress callback.

        Raises:
            Error: If the download fails or cannot be written to disk.
        """
        ...

    def tasks(
        self,
        name: Optional[str] = None,
        workflow: Optional[str] = None,
        status: Optional[str] = None,
        manager: Optional[str] = None,
    ) -> List[Task]:
        """
        Returns a list of tasks available to the user.  The tasks are returned
        as a vector of Task objects.  Tasks represent some workflow within
        EdgeFirst Studio such as trainer or validation sessions.  The managers
        represent where the task is run such as cloud, or user-managed, or
        kubernetes for on-premise installations.

        Args:
            name (Optional[str]): The name of the task to filter by.
            workflow (Optional[str]): The workflow name to filter by.
            status (Optional[str]): The status to filter by.
            manager (Optional[str]): The task manager to filter by.

        Returns:
            List[Task]: A list of Task objects.

        Raises:
            Error: If the server request fails.
        """
        ...

    def task_info(self, task_id: TaskUID) -> TaskInfo:
        """
        Returns detailed information about a specific task.

        Args:
            task_id (Union[TaskID, int, str]): The ID of the task to retrieve.

        Returns:
            TaskInfo: The TaskInfo object containing detailed information.

        Raises:
            Error: If the task does not exist or the request fails.
        """
        ...

    def task_status(self, task_id: TaskUID, status: str) -> Task:
        """
        Updates the task status.

        Args:
            task_id (Union[TaskID, int, str]): The ID of the task to update.
            status (str): The new status to set for the task.

        Returns:
            Task: The updated Task object.

        Raises:
            Error: If the task does not exist or the request fails.
        """
        ...

    def set_stages(
        self, task_id: TaskUID, stages: List[Tuple[str, str]]
    ) -> None:
        """
        Configures the task stages.  Stages are used to show various steps in
        the task execution process.

        Args:
            task_id (Union[TaskID, int, str]): The ID of the task to update.
            stages (Dict[str, str]): A dictionary representing the new stages
                                      for the task.

        Returns:
            None

        Raises:
            Error: If the task does not exist or the request fails.
        """
        ...

    def update_stage(
        self,
        task_id: TaskUID,
        stage: str,
        status: str,
        message: str,
        percentage: int,
    ) -> None:
        """
        Updates a specific stage of a task.

        Args:
            task_id (Union[TaskID, int, str]): The ID of the task to update.
            stage (str): The name of the stage to update.
            status (str): The new status of the stage.
            message (str): A message describing the current state of the stage.
            percentage (int): The completion percentage of the stage (0-100).

        Returns:
            None

        Raises:
            Error: If the task or stage does not exist or the request fails.
        """
        ...


def version() -> str:
    """
    Get the version of the edgefirst_client library.

    Returns:
        The version string (e.g., "0.3.0").

    Examples:
        >>> import edgefirst_client as ec
        >>> print(ec.version())
        0.3.0
    """
    ...


def is_polars_enabled() -> bool:
    """
    Check if the Polars feature is enabled in this build.

    The Polars feature enables DataFrame support for annotations and
    other data structures. It is enabled at compile time with the
    'polars' feature flag.

    Returns:
        True if Polars support is compiled in, False otherwise.

    Examples:
        >>> import edgefirst_client as ec
        >>> if ec.is_polars_enabled():
        ...     df = client.annotations_dataframe(annotation_set_id)
        ... else:
        ...     annotations = client.annotations(annotation_set_id)
    """
    ...
