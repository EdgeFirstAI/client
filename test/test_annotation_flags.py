# SPDX-License-Identifier: Apache-2.0
# Copyright © 2026 Au-Zone Technologies. All Rights Reserved.

import unittest
import warnings


class TestAnnotationFlags(unittest.TestCase):
    def test_ignore_and_exclude(self):
        import edgefirst_client as ec

        ann = ec.Annotation()
        self.assertIsNone(ann.ignore)
        ann.set_ignore(True)
        ann.set_exclude(False)
        self.assertTrue(ann.ignore)
        self.assertFalse(ann.exclude)

    def test_property_setters_and_is_flagged(self):
        import edgefirst_client as ec

        ann = ec.Annotation()
        self.assertFalse(ann.is_flagged())
        ann.ignore = True
        self.assertTrue(ann.ignore)
        self.assertTrue(ann.is_flagged())
        ann.ignore = None
        ann.exclude = True
        self.assertIsNone(ann.ignore)
        self.assertTrue(ann.exclude)
        self.assertTrue(ann.is_flagged())
        ann.exclude = False
        self.assertFalse(ann.is_flagged())

    def test_setters_accept_documented_keyword_names(self):
        import edgefirst_client as ec

        ann = ec.Annotation()
        ann.set_ignore(ignore=True)
        ann.set_exclude(exclude=True)
        self.assertTrue(ann.ignore)
        self.assertTrue(ann.exclude)

    def test_iscrowd_is_deprecated_alias(self):
        import edgefirst_client as ec

        ann = ec.Annotation()
        with warnings.catch_warnings(record=True) as caught:
            warnings.simplefilter("always")
            ann.set_iscrowd(True)
            self.assertTrue(ann.iscrowd)
        self.assertTrue(ann.ignore)
        self.assertEqual(
            sum(issubclass(w.category, DeprecationWarning) for w in caught), 2
        )


if __name__ == "__main__":
    unittest.main()
