#!/usr/bin/env python3
"""
Comprehensive tests for the Parameter class.

This tests all constructors, type conversions, and magic methods.
"""

import unittest
import edgefirst_client as ec


class TestParameter(unittest.TestCase):
    """Test suite for Parameter class."""

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
        self.assertEqual(str(p), "String(hello world)")

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

    def test_equality_integer(self):
        """Test equality comparison for Integer parameters."""
        p = ec.Parameter.integer(42)

        # Should equal same integer
        self.assertTrue(p == 42)

        # Should equal close float (within epsilon)
        self.assertTrue(p == 42.0)

        # Should not equal different value
        self.assertFalse(p == 43)

    def test_equality_real(self):
        """Test equality comparison for Real parameters."""
        p = ec.Parameter.real(0.75)

        # Should equal same float
        self.assertTrue(p == 0.75)

        # Should equal very close float (within epsilon 1e-9)
        self.assertTrue(p == 0.75000000001)
        self.assertTrue(p == 0.74999999999)

        # Should equal equivalent integer
        p_int = ec.Parameter.real(42.0)
        self.assertTrue(p_int == 42)

        # Should not equal different value
        self.assertFalse(p == 0.76)

    def test_equality_boolean(self):
        """Test equality comparison for Boolean parameters."""
        p_true = ec.Parameter.boolean(True)
        p_false = ec.Parameter.boolean(False)

        # Testing Parameter.__eq__ with boolean literals (intentional)
        self.assertTrue(p_true == True)  # noqa: E712
        self.assertTrue(p_false == False)  # noqa: E712
        self.assertFalse(p_true == False)  # noqa: E712
        self.assertFalse(p_false == True)  # noqa: E712

    def test_equality_string(self):
        """Test equality comparison for String parameters."""
        p = ec.Parameter.string("hello")

        self.assertTrue(p == "hello")
        self.assertFalse(p == "world")

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
        self.assertEqual(str(p_str), "String(hello)")

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

    def test_integration_with_metrics(self):
        """
        Test that Parameters work correctly with set_metrics/metrics flow.

        This verifies the fix for SonarCloud python:S1244 issues.
        """
        # This is conceptually what happens in test.py lines 176, 196
        # but we can't easily test it without a real client connection.
        # Instead, we verify the equality works as expected:

        p = ec.Parameter.real(0.75)
        self.assertTrue(p == 0.75)  # This is what test.py does

        # Also verify math.isclose() would work if needed
        import math

        self.assertTrue(math.isclose(float(p), 0.75))


if __name__ == "__main__":
    unittest.main()
