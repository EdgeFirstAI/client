#!/usr/bin/env python3
"""
Comprehensive tests for the Parameter class.

Tests constructors, type conversions, magic methods, and round-trip
conversions between Python native types and Parameter objects.
"""

import unittest
import edgefirst_client as ec


class TestParameter(unittest.TestCase):
    """Test suite for Parameter class."""

    # =========================================================================
    # Constructor Tests
    # =========================================================================

    def test_integer_constructor(self):
        """Test Parameter.integer() constructor."""
        p = ec.Parameter.integer(42)
        self.assertTrue(p.is_integer())
        self.assertFalse(p.is_real())
        self.assertEqual(p.type_name(), "Integer")
        self.assertEqual(int(p), 42)
        self.assertEqual(float(p), 42.0)

    def test_real_constructor(self):
        """Test Parameter.real() constructor."""
        p = ec.Parameter.real(3.14)
        self.assertTrue(p.is_real())
        self.assertFalse(p.is_integer())
        self.assertEqual(p.type_name(), "Real")
        self.assertEqual(float(p), 3.14)
        self.assertEqual(int(p), 3)

    def test_boolean_constructor(self):
        """Test Parameter.boolean() constructor."""
        p_true = ec.Parameter.boolean(True)
        p_false = ec.Parameter.boolean(False)

        self.assertTrue(p_true.is_boolean())
        self.assertEqual(p_true.type_name(), "Boolean")
        self.assertTrue(bool(p_true))
        self.assertFalse(bool(p_false))
        self.assertEqual(int(p_true), 1)
        self.assertEqual(int(p_false), 0)
        self.assertEqual(float(p_true), 1.0)
        self.assertEqual(float(p_false), 0.0)

    def test_string_constructor(self):
        """Test Parameter.string() constructor."""
        p = ec.Parameter.string("hello world")
        self.assertTrue(p.is_string())
        self.assertEqual(p.type_name(), "String")
        self.assertTrue(bool(p))
        self.assertEqual(str(p), "hello world")
        self.assertEqual(repr(p), "String(hello world)")

        p_empty = ec.Parameter.string("")
        self.assertFalse(bool(p_empty))

    def test_array_constructor(self):
        """Test Parameter.array() constructor."""
        p = ec.Parameter.array([1, 2.5, True, "hello"])
        self.assertTrue(p.is_array())
        self.assertEqual(p.type_name(), "Array")
        self.assertTrue(bool(p))

        p_empty = ec.Parameter.array([])
        self.assertFalse(bool(p_empty))

    def test_object_constructor(self):
        """Test Parameter.object() constructor."""
        p = ec.Parameter.object({"key1": 42, "key2": 3.14, "key3": True})
        self.assertTrue(p.is_object())
        self.assertEqual(p.type_name(), "Object")
        self.assertTrue(bool(p))

        p_empty = ec.Parameter.object({})
        self.assertFalse(bool(p_empty))

    def test_nested_structures(self):
        """Test nested arrays and objects."""
        # Nested array
        p_nested_array = ec.Parameter.array([1, [2, 3], {"key": "value"}])
        self.assertTrue(p_nested_array.is_array())

        # Nested object
        p_nested_obj = ec.Parameter.object(
            {"number": 42, "array": [1, 2, 3], "nested": {"inner": "value"}}
        )
        self.assertTrue(p_nested_obj.is_object())

    # =========================================================================
    # Equality Tests
    # =========================================================================

    def test_equality_integer(self):
        """Test equality comparison for Integer parameters."""
        p = ec.Parameter.integer(42)

        # Should equal same integer
        self.assertEqual(p, 42)

        # Should equal close float (within epsilon)
        self.assertEqual(p, 42.0)

        # Should not equal different value
        self.assertNotEqual(p, 43)

    def test_equality_real(self):
        """Test equality comparison for Real parameters."""
        p = ec.Parameter.real(0.75)

        # Should equal same float
        self.assertEqual(p, 0.75)

        # Should equal very close float (within epsilon 1e-9)
        self.assertEqual(p, 0.75000000001)
        self.assertEqual(p, 0.74999999999)

        # Should equal equivalent integer
        p_int = ec.Parameter.real(42.0)
        self.assertEqual(p_int, 42)

        # Should not equal different value
        self.assertNotEqual(p, 0.76)

    def test_equality_boolean(self):
        """Test equality comparison for Boolean parameters."""
        p_true = ec.Parameter.boolean(True)
        p_false = ec.Parameter.boolean(False)

        # Testing Parameter.__eq__ with boolean literals (intentional)
        # Using assertTrue/assertFalse with == to test __eq__ implementation
        self.assertTrue(p_true == True)  # noqa: E712  # NOSONAR
        self.assertFalse(p_false == True)  # noqa: E712  # NOSONAR
        self.assertTrue(p_false == False)  # noqa: E712  # NOSONAR
        self.assertFalse(p_true == False)  # noqa: E712  # NOSONAR

    def test_equality_string(self):
        """Test equality comparison for String parameters."""
        p = ec.Parameter.string("hello")

        self.assertEqual(p, "hello")
        self.assertNotEqual(p, "world")

    # =========================================================================
    # Type Conversion Tests
    # =========================================================================

    def test_type_conversions(self):
        """Test type conversion magic methods."""
        # Integer conversions
        p_int = ec.Parameter.integer(42)
        self.assertEqual(int(p_int), 42)
        self.assertEqual(float(p_int), 42.0)

        # Real conversions
        p_real = ec.Parameter.real(3.14)
        self.assertEqual(int(p_real), 3)
        self.assertEqual(float(p_real), 3.14)

        # Boolean conversions
        p_bool = ec.Parameter.boolean(True)
        self.assertEqual(int(p_bool), 1)
        self.assertEqual(float(p_bool), 1.0)
        self.assertTrue(bool(p_bool))

    def test_type_conversion_errors(self):
        """Test that invalid type conversions raise TypeError."""
        p_str = ec.Parameter.string("hello")
        p_array = ec.Parameter.array([1, 2, 3])
        p_obj = ec.Parameter.object({"key": "value"})

        # String, Array, Object cannot be converted to int
        with self.assertRaises(TypeError):
            int(p_str)
        with self.assertRaises(TypeError):
            int(p_array)
        with self.assertRaises(TypeError):
            int(p_obj)

        # String, Array, Object cannot be converted to float
        with self.assertRaises(TypeError):
            float(p_str)
        with self.assertRaises(TypeError):
            float(p_array)
        with self.assertRaises(TypeError):
            float(p_obj)

    # =========================================================================
    # String Representation Tests
    # =========================================================================

    def test_string_representations(self):
        """Test __str__ and __repr__ methods."""
        p_int = ec.Parameter.integer(42)
        self.assertEqual(str(p_int), "Integer(42)")
        self.assertEqual(repr(p_int), "Integer(42)")

        p_real = ec.Parameter.real(3.14)
        self.assertEqual(str(p_real), "Real(3.14)")

        p_bool = ec.Parameter.boolean(True)
        self.assertEqual(str(p_bool), "Boolean(true)")

        p_str = ec.Parameter.string("hello")
        # String __str__ returns plain value
        self.assertEqual(str(p_str), "hello")
        # String __repr__ is descriptive
        self.assertEqual(repr(p_str), "String(hello)")

    def test_bool_truthiness(self):
        """Test __bool__ method for all types."""
        # Numeric types
        self.assertTrue(bool(ec.Parameter.integer(42)))
        self.assertFalse(bool(ec.Parameter.integer(0)))
        self.assertTrue(bool(ec.Parameter.real(3.14)))
        self.assertFalse(bool(ec.Parameter.real(0.0)))

        # Boolean type
        self.assertTrue(bool(ec.Parameter.boolean(True)))
        self.assertFalse(bool(ec.Parameter.boolean(False)))

        # String type
        self.assertTrue(bool(ec.Parameter.string("hello")))
        self.assertFalse(bool(ec.Parameter.string("")))

        # Array type
        self.assertTrue(bool(ec.Parameter.array([1, 2, 3])))
        self.assertFalse(bool(ec.Parameter.array([])))

        # Object type
        self.assertTrue(bool(ec.Parameter.object({"key": "value"})))
        self.assertFalse(bool(ec.Parameter.object({})))

    def test_variant_type(self):
        """Test variant_type() returns correct type names."""
        self.assertEqual(ec.Parameter.integer(42).variant_type(), "Integer")
        self.assertEqual(ec.Parameter.real(3.14).variant_type(), "Real")
        self.assertEqual(ec.Parameter.boolean(True).variant_type(), "Boolean")
        self.assertEqual(ec.Parameter.string("test").variant_type(), "String")
        self.assertEqual(ec.Parameter.array([]).variant_type(), "Array")
        self.assertEqual(ec.Parameter.object({}).variant_type(), "Object")

    # =========================================================================
    # Integration Tests
    # =========================================================================

    def test_integration_with_metrics(self):
        """
        Test that Parameters work correctly with set_metrics/metrics flow.

        This verifies the fix for SonarCloud python:S1244 issues.
        """
        # This is conceptually what happens in test.py lines 176, 196
        # but we can't easily test it without a real client connection.
        # Instead, we verify the equality works as expected:

        p = ec.Parameter.real(0.75)
        self.assertEqual(p, 0.75)  # This is what test.py does

        # Also verify math.isclose() would work if needed
        import math

        self.assertTrue(math.isclose(float(p), 0.75))

    # =========================================================================
    # Round-Trip Conversion Tests
    # =========================================================================

    def test_integer_roundtrip(self):
        """Test int -> Parameter -> int preserves value and type."""
        original = 42
        param = ec.Parameter.integer(original)

        # Verify it's an Integer parameter
        self.assertTrue(param.is_integer())
        self.assertEqual(param.type_name(), "Integer")

        # Extract back to Python int
        extracted = param.as_integer()
        self.assertIsNotNone(extracted)
        assert extracted is not None
        self.assertEqual(extracted, original)
        self.assertIsInstance(extracted, int)

        # Verify wrong type returns None
        self.assertIsNone(param.as_real())
        self.assertIsNone(param.as_boolean())
        self.assertIsNone(param.as_string())

    def test_real_roundtrip(self):
        """Test float -> Parameter -> float preserves value and type."""
        original = 3.14159
        param = ec.Parameter.real(original)

        # Verify it's a Real parameter
        self.assertTrue(param.is_real())
        self.assertEqual(param.type_name(), "Real")

        # Extract back to Python float
        extracted = param.as_real()
        self.assertIsNotNone(extracted)
        assert extracted is not None
        self.assertAlmostEqual(extracted, original, places=10)
        self.assertIsInstance(extracted, float)

        # Verify wrong type returns None
        self.assertIsNone(param.as_integer())
        self.assertIsNone(param.as_boolean())
        self.assertIsNone(param.as_string())

    def test_boolean_roundtrip(self):
        """Test bool -> Parameter -> bool preserves value and type."""
        for original in [True, False]:
            with self.subTest(value=original):
                param = ec.Parameter.boolean(original)

                # Verify it's a Boolean parameter
                self.assertTrue(param.is_boolean())
                self.assertEqual(param.type_name(), "Boolean")

                # Extract back to Python bool
                extracted = param.as_boolean()
                self.assertIsNotNone(extracted)
                assert extracted is not None
                self.assertEqual(extracted, original)
                self.assertIsInstance(extracted, bool)

                # Verify wrong type returns None
                self.assertIsNone(param.as_integer())
                self.assertIsNone(param.as_real())
                self.assertIsNone(param.as_string())

    def test_string_roundtrip(self):
        """Test str -> Parameter -> str preserves value and type."""
        original = "hello world"
        param = ec.Parameter.string(original)

        # Verify it's a String parameter
        self.assertTrue(param.is_string())
        self.assertEqual(param.type_name(), "String")

        # Extract back to Python str
        extracted = param.as_string()
        self.assertIsNotNone(extracted)
        assert extracted is not None
        self.assertEqual(extracted, original)
        self.assertIsInstance(extracted, str)

        # Verify wrong type returns None
        self.assertIsNone(param.as_integer())
        self.assertIsNone(param.as_real())
        self.assertIsNone(param.as_boolean())

    def test_array_simple_roundtrip(self):
        """Test list -> Parameter -> list preserves values and types."""
        # Convert to Parameter array
        param = ec.Parameter.array(
            [
                ec.Parameter.integer(42),
                ec.Parameter.real(3.14),
                ec.Parameter.boolean(True),
                ec.Parameter.string("test"),
            ]
        )

        # Verify it's an Array parameter
        self.assertTrue(param.is_array())
        self.assertEqual(param.type_name(), "Array")

        # Extract back to Python list
        extracted = param.as_array()
        self.assertIsNotNone(extracted)
        assert extracted is not None
        self.assertIsInstance(extracted, list)
        self.assertEqual(len(extracted), 4)

        # Verify each element
        self.assertEqual(extracted[0], 42)
        self.assertIsInstance(extracted[0], int)

        self.assertAlmostEqual(extracted[1], 3.14, places=10)
        self.assertIsInstance(extracted[1], float)

        # Testing value equality (type already verified by assertIsInstance)
        self.assertEqual(extracted[2], True)  # noqa: E712  # NOSONAR
        self.assertIsInstance(extracted[2], bool)

        self.assertEqual(extracted[3], "test")
        self.assertIsInstance(extracted[3], str)

    def test_array_nested_roundtrip(self):
        """Test nested arrays preserve structure."""
        # Create nested array: [[1, 2], [3, 4]]
        param = ec.Parameter.array(
            [
                ec.Parameter.array(
                    [
                        ec.Parameter.integer(1),
                        ec.Parameter.integer(2),
                    ]
                ),
                ec.Parameter.array(
                    [
                        ec.Parameter.integer(3),
                        ec.Parameter.integer(4),
                    ]
                ),
            ]
        )

        # Extract and verify
        extracted = param.as_array()
        self.assertIsNotNone(extracted)
        assert extracted is not None
        self.assertEqual(len(extracted), 2)

        # Verify nested structure
        self.assertIsInstance(extracted[0], list)
        self.assertIsInstance(extracted[1], list)
        self.assertEqual(extracted[0], [1, 2])
        self.assertEqual(extracted[1], [3, 4])

    def test_object_simple_roundtrip(self):
        """Test dict -> Parameter -> dict preserves values and types."""
        # Create object with various types
        param = ec.Parameter.object(
            {
                "count": ec.Parameter.integer(42),
                "ratio": ec.Parameter.real(3.14),
                "enabled": ec.Parameter.boolean(True),
                "name": ec.Parameter.string("test"),
            }
        )

        # Verify it's an Object parameter
        self.assertTrue(param.is_object())
        self.assertEqual(param.type_name(), "Object")

        # Extract back to Python dict
        extracted = param.as_object()
        self.assertIsNotNone(extracted)
        assert extracted is not None
        self.assertIsInstance(extracted, dict)
        self.assertEqual(len(extracted), 4)

        # Verify each value
        self.assertEqual(extracted["count"], 42)
        self.assertIsInstance(extracted["count"], int)

        self.assertAlmostEqual(extracted["ratio"], 3.14, places=10)
        self.assertIsInstance(extracted["ratio"], float)

        # Testing value equality (type already verified by assertIsInstance)
        self.assertEqual(extracted["enabled"], True)  # noqa: E712  # NOSONAR
        self.assertIsInstance(extracted["enabled"], bool)

        self.assertEqual(extracted["name"], "test")
        self.assertIsInstance(extracted["name"], str)

    def test_object_nested_roundtrip(self):
        """Test nested objects preserve structure."""
        # Create nested object
        param = ec.Parameter.object(
            {
                "config": ec.Parameter.object(
                    {
                        "timeout": ec.Parameter.integer(30),
                        "retries": ec.Parameter.integer(3),
                    }
                ),
                "data": ec.Parameter.array(
                    [
                        ec.Parameter.string("a"),
                        ec.Parameter.string("b"),
                    ]
                ),
            }
        )

        # Extract and verify
        extracted = param.as_object()
        self.assertIsNotNone(extracted)
        assert extracted is not None
        self.assertEqual(len(extracted), 2)

        # Verify nested structure
        self.assertIsInstance(extracted["config"], dict)
        self.assertEqual(extracted["config"]["timeout"], 30)
        self.assertEqual(extracted["config"]["retries"], 3)

        self.assertIsInstance(extracted["data"], list)
        self.assertEqual(extracted["data"], ["a", "b"])

    def test_complex_nested_structure(self):
        """Test deeply nested structure with mixed types."""
        # Create complex nested structure
        param = ec.Parameter.object(
            {
                "version": ec.Parameter.integer(1),
                "settings": ec.Parameter.object(
                    {
                        "timeout": ec.Parameter.real(30.5),
                        "retries": ec.Parameter.integer(3),
                        "features": ec.Parameter.array(
                            [
                                ec.Parameter.string("feature1"),
                                ec.Parameter.string("feature2"),
                            ]
                        ),
                        "flags": ec.Parameter.object(
                            {
                                "debug": ec.Parameter.boolean(True),
                                "verbose": ec.Parameter.boolean(False),
                            }
                        ),
                    }
                ),
                "data": ec.Parameter.array(
                    [
                        ec.Parameter.integer(1),
                        ec.Parameter.integer(2),
                        ec.Parameter.array(
                            [
                                ec.Parameter.integer(3),
                                ec.Parameter.integer(4),
                            ]
                        ),
                    ]
                ),
            }
        )

        # Extract and verify entire structure
        extracted = param.as_object()
        self.assertIsNotNone(extracted)
        assert extracted is not None

        # Verify top level
        self.assertEqual(extracted["version"], 1)

        # Verify settings object
        settings = extracted["settings"]
        self.assertEqual(settings["timeout"], 30.5)
        self.assertEqual(settings["retries"], 3)
        self.assertEqual(settings["features"], ["feature1", "feature2"])
        # Testing value equality (type preservation verified elsewhere)
        self.assertTrue(settings["flags"]["debug"])
        self.assertFalse(settings["flags"]["verbose"])

        # Verify data array with nested array
        data = extracted["data"]
        self.assertEqual(data[0], 1)
        self.assertEqual(data[1], 2)
        self.assertEqual(data[2], [3, 4])

    def test_empty_collections(self):
        """Test empty arrays and objects."""
        # Empty array
        empty_array = ec.Parameter.array([])
        self.assertTrue(empty_array.is_array())
        extracted_array = empty_array.as_array()
        self.assertIsNotNone(extracted_array)
        assert extracted_array is not None
        self.assertEqual(extracted_array, [])

        # Empty object
        empty_object = ec.Parameter.object({})
        self.assertTrue(empty_object.is_object())
        extracted_object = empty_object.as_object()
        self.assertIsNotNone(extracted_object)
        assert extracted_object is not None
        self.assertEqual(extracted_object, {})

    def test_type_preservation_through_conversion(self):
        """Verify types are strictly preserved, not coerced."""
        # Integer stays integer
        int_param = ec.Parameter.integer(42)
        self.assertIsNone(int_param.as_real())
        self.assertIsNone(int_param.as_string())

        # Real stays real
        real_param = ec.Parameter.real(3.14)
        self.assertIsNone(real_param.as_integer())
        self.assertIsNone(real_param.as_string())

        # String stays string
        str_param = ec.Parameter.string("42")
        self.assertIsNone(str_param.as_integer())
        self.assertIsNone(str_param.as_real())

        # Boolean stays boolean
        bool_param = ec.Parameter.boolean(True)
        self.assertIsNone(bool_param.as_integer())
        self.assertIsNone(bool_param.as_string())

    # =========================================================================
    # Pythonic Dict/List-like API Tests
    # =========================================================================

    def test_object_getitem(self):
        """Test Object parameter supports dict-like .get() access.

        Note: Bracket indexing (param['key']) is not supported due to PyO3
        limitations with enum variants. Use .get('key') instead.
        """
        param = ec.Parameter.object(
            {
                "count": ec.Parameter.integer(42),
                "ratio": ec.Parameter.real(3.14),
                "enabled": ec.Parameter.boolean(True),
                "name": ec.Parameter.string("test"),
            }
        )

        # Test successful key access with .get()
        self.assertEqual(param.get("count"), 42)
        self.assertAlmostEqual(param.get("ratio"), 3.14, places=10)
        self.assertTrue(param.get("enabled"))
        self.assertEqual(param.get("name"), "test")

        # Test None return for missing key
        self.assertIsNone(param.get("missing_key"))

    def test_object_get_method(self):
        """Test Object parameter supports .get() method like dict."""
        param = ec.Parameter.object(
            {
                "model": ec.Parameter.string("yolov5"),
                "detection": ec.Parameter.boolean(True),
            }
        )

        # Test successful key access
        self.assertEqual(param.get("model"), "yolov5")
        self.assertTrue(param.get("detection"))

        # Test default value for missing key
        self.assertIsNone(param.get("missing"))
        self.assertEqual(param.get("missing", "default"), "default")
        self.assertEqual(param.get("missing", 99), 99)

    def test_object_get_nested(self):
        """Test .get() works with nested structures."""
        param = ec.Parameter.object(
            {
                "model": ec.Parameter.object(
                    {
                        "detection": ec.Parameter.boolean(True),
                        "config": ec.Parameter.object(
                            {
                                "threshold": ec.Parameter.real(0.5),
                            }
                        ),
                    }
                ),
            }
        )

        # Get nested object
        model = param.get("model")
        self.assertIsNotNone(model)
        assert model is not None
        self.assertIsInstance(model, dict)
        self.assertTrue(model["detection"])

        # Chain .get() calls - this should work now!
        # This was the user's original failing code:
        # self.detection = trainer.model_params.get('model').get('detection')
        model_dict = param.get("model")
        assert model_dict is not None
        # model_dict is now a Python dict, not a Parameter
        detection = model_dict.get("detection")
        self.assertTrue(detection)

    def test_array_iteration(self):
        """Test Array parameter can be converted to native Python list.

        Note: Direct indexing (param[0]) is not supported due to PyO3
        limitations. Use .as_array() to get a native Python list.
        """
        param = ec.Parameter.array(
            [
                ec.Parameter.integer(10),
                ec.Parameter.real(20.5),
                ec.Parameter.string("thirty"),
            ]
        )

        # Convert to Python list for indexing
        arr = param.as_array()
        self.assertEqual(arr[0], 10)
        self.assertAlmostEqual(arr[1], 20.5, places=10)
        self.assertEqual(arr[2], "thirty")

        # Test bounds on converted list
        self.assertEqual(len(arr), 3)
        with self.assertRaises(IndexError):
            _ = arr[3]

    def test_string_str_vs_repr(self):
        """Test __str__ returns plain string, __repr__ is descriptive."""
        param = ec.Parameter.string("hello")

        # __str__ should return plain string value
        self.assertEqual(str(param), "hello")

        # __repr__ should return descriptive format
        self.assertEqual(repr(param), "String(hello)")

        # This addresses the user's issue of having to parse "String(...)"
        modelname = str(param)
        self.assertEqual(modelname, "hello")
        # No need for: modelname.removeprefix("String(").removesuffix(")")

    def test_object_keys_values_items(self):
        """Test Object dict-like .keys(), .values(), .items()."""
        param = ec.Parameter.object(
            {
                "a": ec.Parameter.integer(1),
                "b": ec.Parameter.integer(2),
                "c": ec.Parameter.integer(3),
            }
        )

        # Test keys()
        keys = param.keys()
        self.assertIsInstance(keys, list)
        self.assertEqual(set(keys), {"a", "b", "c"})

        # Test values()
        values = param.values()
        self.assertIsInstance(values, list)
        self.assertEqual(set(values), {1, 2, 3})

        # Test items()
        items = param.items()
        self.assertIsInstance(items, list)
        items_dict = dict(items)
        self.assertEqual(items_dict, {"a": 1, "b": 2, "c": 3})

    def test_len_for_collections(self):
        """Test length via .keys() for Object parameters.

        Note: len() is not supported due to PyO3 limitations.
        Use len(obj.keys()) or len(obj.as_object()) instead.
        """
        # Object length via keys()
        obj = ec.Parameter.object(
            {
                "a": ec.Parameter.integer(1),
                "b": ec.Parameter.integer(2),
            }
        )
        self.assertEqual(len(obj.keys()), 2)
        self.assertEqual(len(obj.as_object()), 2)

        # Array length via as_array()
        arr = ec.Parameter.array([1, 2, 3, 4, 5])
        self.assertEqual(len(arr.as_array()), 5)

        # Empty collections
        self.assertEqual(len(ec.Parameter.array([]).as_array()), 0)
        self.assertEqual(len(ec.Parameter.object({}).keys()), 0)

    def test_contains_for_collections(self):
        """Test membership checking via .keys() for Object parameters.

        Note: 'in' operator is not supported due to PyO3 limitations.
        Use 'key in obj.keys()' instead.
        """
        # Object contains (check keys)
        obj = ec.Parameter.object(
            {
                "model": ec.Parameter.string("yolov5"),
                "detection": ec.Parameter.boolean(True),
            }
        )
        keys = obj.keys()
        self.assertIn("model", keys)
        self.assertIn("detection", keys)
        self.assertNotIn("missing", keys)

        # Array contains (check values in converted list)
        arr = ec.Parameter.array([10, 20, 30])
        arr_list = arr.as_array()
        self.assertIn(10, arr_list)
        self.assertIn(20, arr_list)
        self.assertIn(30, arr_list)
        self.assertNotIn(99, arr_list)

    def test_pythonic_workflow_example(self):
        """Test real-world Pythonic workflow from user feedback.

        This demonstrates the recommended patterns for working with
        Parameter objects in a Pythonic way.
        """
        # Simulate trainer.model_params structure
        trainer_params = ec.Parameter.object(
            {
                "model": ec.Parameter.object(
                    {
                        "detection": ec.Parameter.boolean(True),
                        "name": ec.Parameter.string("yolov5"),
                        "threshold": ec.Parameter.real(0.75),
                    }
                ),
                "epochs": ec.Parameter.integer(100),
            }
        )

        # Recommended: Chained .get() calls (Pythonic pattern)
        detection = trainer_params.get("model").get("detection")
        self.assertTrue(detection)

        # Get nested values
        model_name = trainer_params.get("model").get("name")
        self.assertEqual(model_name, "yolov5")

        # Test .get() with default
        missing = trainer_params.get("missing_key", "default_value")
        self.assertEqual(missing, "default_value")

        # Test str() without needing to parse
        # Old way (user had to do this):
        # modelname = str(validation.params["model"])
        # if "String" in modelname:
        #     modelname = modelname.removeprefix("String(").removesuffix(")")

        # New way - just works:
        name_param = ec.Parameter.string("yolov5")
        clean_name = str(name_param)
        self.assertEqual(clean_name, "yolov5")
        self.assertNotIn("String(", clean_name)

        # Using .keys(), .values(), .items() for iteration
        model_obj = trainer_params.get("model")
        self.assertIn("detection", model_obj.keys())
        self.assertEqual(len(model_obj.keys()), 3)


if __name__ == "__main__":
    unittest.main()
