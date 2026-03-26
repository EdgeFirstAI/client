// SPDX-License-Identifier: Apache-2.0
// Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

//! Integration tests for COCO format support.

#[cfg(test)]
mod integration_tests {
    use super::super::*;
    use tempfile::TempDir;

    /// Test full round-trip: COCO JSON → EdgeFirst Arrow → COCO JSON
    #[cfg(feature = "polars")]
    #[tokio::test]
    async fn test_coco_arrow_roundtrip() {
        let temp_dir = TempDir::new().unwrap();

        // Create realistic COCO dataset with multiple objects
        let original = CocoDataset {
            info: CocoInfo {
                year: Some(2025),
                version: Some("1.0".to_string()),
                description: Some("Test dataset".to_string()),
                ..Default::default()
            },
            images: vec![
                CocoImage {
                    id: 1,
                    width: 640,
                    height: 480,
                    file_name: "image_001.jpg".to_string(),
                    ..Default::default()
                },
                CocoImage {
                    id: 2,
                    width: 800,
                    height: 600,
                    file_name: "image_002.jpg".to_string(),
                    ..Default::default()
                },
            ],
            categories: vec![
                CocoCategory {
                    id: 1,
                    name: "person".to_string(),
                    supercategory: Some("human".to_string()),
                    ..Default::default()
                },
                CocoCategory {
                    id: 2,
                    name: "car".to_string(),
                    supercategory: Some("vehicle".to_string()),
                    ..Default::default()
                },
            ],
            annotations: vec![
                // Two annotations on first image
                CocoAnnotation {
                    id: 1,
                    image_id: 1,
                    category_id: 1,
                    bbox: [100.0, 50.0, 200.0, 300.0],
                    area: 60000.0,
                    iscrowd: 0,
                    segmentation: Some(CocoSegmentation::Polygon(vec![vec![
                        100.0, 50.0, 300.0, 50.0, 300.0, 350.0, 100.0, 350.0,
                    ]])),
                },
                CocoAnnotation {
                    id: 2,
                    image_id: 1,
                    category_id: 2,
                    bbox: [400.0, 200.0, 150.0, 100.0],
                    area: 15000.0,
                    iscrowd: 0,
                    segmentation: None,
                },
                // One annotation on second image
                CocoAnnotation {
                    id: 3,
                    image_id: 2,
                    category_id: 1,
                    bbox: [50.0, 100.0, 300.0, 400.0],
                    area: 120000.0,
                    iscrowd: 0,
                    segmentation: Some(CocoSegmentation::Polygon(vec![vec![
                        50.0, 100.0, 350.0, 100.0, 350.0, 500.0, 50.0, 500.0,
                    ]])),
                },
            ],
            licenses: vec![],
        };

        // Write original COCO JSON
        let original_path = temp_dir.path().join("original.json");
        let writer = CocoWriter::new();
        writer.write_json(&original, &original_path).unwrap();

        // Convert to Arrow
        let arrow_path = temp_dir.path().join("converted.arrow");
        let options = CocoToArrowOptions {
            include_masks: true,
            group: Some("train".to_string()),
            ..Default::default()
        };

        let count = coco_to_arrow(&original_path, &arrow_path, &options, None)
            .await
            .unwrap();

        assert_eq!(count, 3); // 3 annotations
        assert!(arrow_path.exists());

        // Convert back to COCO
        let restored_path = temp_dir.path().join("restored.json");
        let options = ArrowToCocoOptions {
            include_masks: true,
            ..Default::default()
        };

        arrow_to_coco(&arrow_path, &restored_path, &options, None)
            .await
            .unwrap();

        // Read restored and compare
        let reader = CocoReader::new();
        let restored = reader.read_json(&restored_path).unwrap();

        // Verify counts match
        assert_eq!(restored.annotations.len(), original.annotations.len());
        assert_eq!(restored.categories.len(), original.categories.len());

        // Verify category names preserved
        let original_cats: std::collections::HashSet<_> =
            original.categories.iter().map(|c| &c.name).collect();
        let restored_cats: std::collections::HashSet<_> =
            restored.categories.iter().map(|c| &c.name).collect();
        assert_eq!(original_cats, restored_cats);
    }

    /// Test bbox conversion accuracy
    #[test]
    fn test_bbox_conversion_precision() {
        let test_cases = vec![
            // (coco_bbox, image_width, image_height)
            ([0.0, 0.0, 640.0, 480.0], 640, 480), // Full image
            ([100.0, 100.0, 200.0, 200.0], 1000, 1000), // Center region
            ([0.5, 0.5, 1.0, 1.0], 100, 100),     // Sub-pixel precision
            ([320.0, 240.0, 100.0, 75.0], 640, 480), // Typical detection
        ];

        for (bbox, w, h) in test_cases {
            let box2d = coco_bbox_to_box2d(&bbox, w, h);
            let restored = box2d_to_coco_bbox(&box2d, w, h);

            for i in 0..4 {
                assert!(
                    (bbox[i] - restored[i]).abs() < 1.0,
                    "Bbox mismatch at index {}: {} vs {} (original: {:?}, restored: {:?})",
                    i,
                    bbox[i],
                    restored[i],
                    bbox,
                    restored
                );
            }
        }
    }

    /// Test polygon segmentation round-trip
    #[test]
    fn test_polygon_roundtrip_precision() {
        let original = vec![
            vec![
                10.0, 20.0, 100.0, 20.0, 100.0, 100.0, 50.0, 120.0, 10.0, 100.0,
            ],
            vec![200.0, 200.0, 250.0, 200.0, 250.0, 250.0, 200.0, 250.0],
        ];

        let image_w = 400;
        let image_h = 400;

        let polygon = coco_polygon_to_polygon(&original, image_w, image_h);
        let restored = polygon_to_coco_polygon(&polygon, image_w, image_h);

        // Verify structure preserved
        assert_eq!(original.len(), restored.len());

        for (orig_poly, rest_poly) in original.iter().zip(restored.iter()) {
            assert_eq!(orig_poly.len(), rest_poly.len());

            for i in 0..orig_poly.len() {
                assert!(
                    (orig_poly[i] - rest_poly[i]).abs() < 1.0,
                    "Polygon mismatch at index {}: {} vs {}",
                    i,
                    orig_poly[i],
                    rest_poly[i]
                );
            }
        }
    }

    /// Test RLE decoding
    #[test]
    fn test_rle_decode_basic() {
        // 4x4 image with a 2x2 square in the top-left
        // Column-major layout:
        // Col 0: [1,1,0,0] → count: 0, 2, 2
        // Col 1: [1,1,0,0] → count: 0, 2, 2
        // Col 2: [0,0,0,0] → count: 4, 0
        // Col 3: [0,0,0,0] → count: 4, 0
        // Combined: [0, 4, 4, 8] = bg:0, fg:4, bg:4, fg:0, bg:8? No...
        //
        // Let's think more carefully:
        // Column 0, rows 0-3: 1,1,0,0 (2 fg at positions 0,1)
        // Column 1, rows 0-3: 1,1,0,0 (2 fg at positions 4,5)
        // Column 2, rows 0-3: 0,0,0,0 (all bg)
        // Column 3, rows 0-3: 0,0,0,0 (all bg)
        //
        // In column-major flat order: [1,1,0,0, 1,1,0,0, 0,0,0,0, 0,0,0,0]
        // positions 0-1: fg, 2-3: bg, 4-5: fg, 6-7: bg, 8-15: bg
        // RLE (starting with bg): [0, 2, 2, 2, 10]
        //
        // Actually, the RLE counts should be: 0 (bg), 2 (fg), 2 (bg), 2 (fg), 10 (bg) =
        // sum 16 ✓

        let rle = CocoRle {
            counts: vec![0, 2, 2, 2, 10],
            size: [4, 4], // height, width
        };

        let (mask, height, width) = decode_rle(&rle).unwrap();

        assert_eq!(height, 4);
        assert_eq!(width, 4);
        assert_eq!(mask.len(), 16);

        // Check the expected pattern in row-major order
        // Row 0: [1,1,0,0]
        // Row 1: [1,1,0,0]
        // Row 2: [0,0,0,0]
        // Row 3: [0,0,0,0]
        assert_eq!(mask[0], 1); // (0,0)
        assert_eq!(mask[1], 1); // (1,0)
        assert_eq!(mask[2], 0); // (2,0)
        assert_eq!(mask[4], 1); // (0,1)
        assert_eq!(mask[5], 1); // (1,1)
        assert_eq!(mask[8], 0); // (0,2)
    }

    /// Test COCO dataset parsing with real-world structure
    #[test]
    fn test_coco_dataset_parsing() {
        let json = r#"{
            "info": {
                "year": 2017,
                "version": "1.0",
                "description": "COCO 2017 Dataset"
            },
            "licenses": [
                {"id": 1, "name": "CC BY 4.0", "url": "https://creativecommons.org/licenses/by/4.0/"}
            ],
            "images": [
                {"id": 397133, "width": 640, "height": 427, "file_name": "000000397133.jpg"}
            ],
            "annotations": [
                {
                    "id": 1768,
                    "image_id": 397133,
                    "category_id": 18,
                    "bbox": [473.07, 395.93, 38.65, 28.67],
                    "area": 702.1,
                    "iscrowd": 0,
                    "segmentation": [[510.66, 423.01, 511.72, 420.03, 510.45, 416.17]]
                }
            ],
            "categories": [
                {"id": 18, "name": "dog", "supercategory": "animal"}
            ]
        }"#;

        let dataset: CocoDataset = serde_json::from_str(json).unwrap();

        assert_eq!(dataset.info.year, Some(2017));
        assert_eq!(dataset.images.len(), 1);
        assert_eq!(dataset.images[0].id, 397133);
        assert_eq!(dataset.annotations.len(), 1);
        assert_eq!(dataset.annotations[0].id, 1768);
        assert_eq!(dataset.categories.len(), 1);
        assert_eq!(dataset.categories[0].name, "dog");

        // Check segmentation
        match &dataset.annotations[0].segmentation {
            Some(CocoSegmentation::Polygon(polys)) => {
                assert_eq!(polys.len(), 1);
                assert_eq!(polys[0].len(), 6);
            }
            _ => panic!("Expected polygon segmentation"),
        }
    }

    /// Test contour extraction from binary mask
    #[test]
    fn test_mask_to_contours_simple_square() {
        // 7x7 image with a 3x3 filled square in the center
        #[rustfmt::skip]
        let mask: Vec<u8> = vec![
            0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0,
            0, 0, 1, 1, 1, 0, 0,
            0, 0, 1, 1, 1, 0, 0,
            0, 0, 1, 1, 1, 0, 0,
            0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0,
        ];

        let contours = mask_to_contours(&mask, 7, 7);

        assert!(!contours.is_empty(), "Should find at least one contour");
        assert!(
            contours[0].len() >= 3,
            "Contour should have at least 3 points"
        );

        // Verify all contour points are within the square region
        for (x, y) in &contours[0] {
            assert!(
                *x >= 2.0 && *x <= 4.0 && *y >= 2.0 && *y <= 4.0,
                "Contour point ({}, {}) outside expected square region",
                x,
                y
            );
        }
    }

    /// Test RLE to mask conversion with contour extraction
    #[test]
    fn test_rle_to_mask_with_contours() {
        // Create RLE for a simple rectangle
        // 10x10 image, rectangle from (2,2) to (7,7)
        // In column-major order, this is:
        // Columns 0-1: all bg (20 pixels each, 40 total = counts[0])
        // Columns 2-7: 2 bg, 6 fg, 2 bg per column = 6 columns
        // Columns 8-9: all bg (20 pixels total)

        // Actually let's make it simpler - 10x10 with a 6x6 filled square
        // Rectangle from row 2, col 2 to row 7, col 7 (6x6 = 36 pixels)
        let _height = 10u32;
        let _width = 10u32;

        // Build RLE manually for this pattern
        // Column 0: all bg (10)
        // Column 1: all bg (10)
        // Column 2: 2 bg, 6 fg, 2 bg = start with prev bg
        // ...and so on

        // For simplicity, let's use a stripe pattern that's easier to verify
        // 10x10 with rows 3-6 (4 rows) being foreground
        // In column-major: each column has bg=3, fg=4, bg=3

        // First column: 3 bg + 4 fg + 3 bg
        // RLE: [3, 4, 3, 4, 3, 4, ...] for 10 columns
        // Total: 10 * 10 = 100 pixels

        // Actually, RLE is cumulative across the whole image
        // Let me build: first 3 bg, then 4 fg, then 6 bg (to next column), etc.
        // This gets complex. Let's just test with a known working pattern.

        // Simple: horizontal stripe in middle
        // Rows 0-2: bg, rows 3-6: fg, rows 7-9: bg
        // In column major: col 0 = [bg bg bg fg fg fg fg bg bg bg]
        // counts: 3 fg, 4 bg, 3 bg = [3, 4, 3] per column

        // For 10 columns, column-major layout:
        // Total = 100
        // Each column: 3 bg, 4 fg, 3 bg (but they merge across columns)

        // Let me just use the RLE from our test_decode_rle_simple
        let rle = CocoRle {
            counts: vec![0, 2, 2, 2, 10], // 2x2 square in top-left of 4x4
            size: [4, 4],
        };

        let polygon_result = coco_rle_to_polygon(&rle, 4, 4);
        assert!(polygon_result.is_ok(), "RLE to polygon should succeed");

        let polygon = polygon_result.unwrap();
        assert!(
            !polygon.rings.is_empty(),
            "Should extract at least one ring from RLE"
        );

        // Check polygon has reasonable number of points
        let total_points: usize = polygon.rings.iter().map(|p| p.len()).sum();
        assert!(
            total_points >= 3,
            "Should have at least 3 points, got {}",
            total_points
        );
    }

    /// Test CocoIndex efficient lookups
    #[test]
    fn test_coco_index_lookups() {
        let dataset = CocoDataset {
            images: vec![
                CocoImage {
                    id: 1,
                    width: 640,
                    height: 480,
                    file_name: "a.jpg".to_string(),
                    ..Default::default()
                },
                CocoImage {
                    id: 2,
                    width: 800,
                    height: 600,
                    file_name: "b.jpg".to_string(),
                    ..Default::default()
                },
            ],
            categories: vec![
                CocoCategory {
                    id: 10,
                    name: "zebra".to_string(),
                    supercategory: None,
                    ..Default::default()
                },
                CocoCategory {
                    id: 20,
                    name: "apple".to_string(),
                    supercategory: None,
                    ..Default::default()
                },
            ],
            annotations: vec![
                CocoAnnotation {
                    id: 100,
                    image_id: 1,
                    category_id: 10,
                    bbox: [0.0; 4],
                    ..Default::default()
                },
                CocoAnnotation {
                    id: 101,
                    image_id: 1,
                    category_id: 20,
                    bbox: [0.0; 4],
                    ..Default::default()
                },
                CocoAnnotation {
                    id: 102,
                    image_id: 2,
                    category_id: 10,
                    bbox: [0.0; 4],
                    ..Default::default()
                },
            ],
            ..Default::default()
        };

        let index = CocoIndex::from_dataset(&dataset);

        // Test image lookup
        assert!(index.images.contains_key(&1));
        assert!(!index.images.contains_key(&999));

        // Test category lookup
        assert_eq!(index.label_name(10), Some("zebra"));
        assert_eq!(index.label_name(20), Some("apple"));
        assert_eq!(index.label_name(999), None);

        // Test source-faithful label indices (category_id preserved)
        assert_eq!(index.label_index(20), Some(20)); // apple
        assert_eq!(index.label_index(10), Some(10)); // zebra

        // Test annotations by image
        assert_eq!(index.annotations_for_image(1).len(), 2);
        assert_eq!(index.annotations_for_image(2).len(), 1);
        assert_eq!(index.annotations_for_image(999).len(), 0);
    }

    /// Test that COCO→Arrow→COCO round-trip preserves non-contiguous category IDs.
    #[cfg(feature = "polars")]
    #[tokio::test]
    async fn test_coco_arrow_roundtrip_preserves_category_id() {
        let temp_dir = TempDir::new().unwrap();

        // Create a COCO dataset with non-contiguous category IDs (gap at 2)
        let original = CocoDataset {
            info: CocoInfo {
                year: Some(2025),
                version: Some("1.0".to_string()),
                description: Some("Category ID preservation test".to_string()),
                ..Default::default()
            },
            images: vec![CocoImage {
                id: 1,
                width: 640,
                height: 480,
                file_name: "test_image.jpg".to_string(),
                ..Default::default()
            }],
            categories: vec![
                CocoCategory {
                    id: 1,
                    name: "person".to_string(),
                    supercategory: Some("human".to_string()),
                    ..Default::default()
                },
                CocoCategory {
                    id: 3,
                    name: "car".to_string(),
                    supercategory: Some("vehicle".to_string()),
                    ..Default::default()
                },
                CocoCategory {
                    id: 90,
                    name: "toothbrush".to_string(),
                    supercategory: Some("indoor".to_string()),
                    ..Default::default()
                },
            ],
            annotations: vec![
                CocoAnnotation {
                    id: 1,
                    image_id: 1,
                    category_id: 1,
                    bbox: [100.0, 50.0, 200.0, 300.0],
                    area: 60000.0,
                    iscrowd: 0,
                    segmentation: None,
                },
                CocoAnnotation {
                    id: 2,
                    image_id: 1,
                    category_id: 3,
                    bbox: [400.0, 200.0, 150.0, 100.0],
                    area: 15000.0,
                    iscrowd: 0,
                    segmentation: None,
                },
                CocoAnnotation {
                    id: 3,
                    image_id: 1,
                    category_id: 90,
                    bbox: [10.0, 10.0, 30.0, 80.0],
                    area: 2400.0,
                    iscrowd: 0,
                    segmentation: None,
                },
            ],
            licenses: vec![],
        };

        // Write original COCO JSON
        let original_path = temp_dir.path().join("original.json");
        let writer = CocoWriter::new();
        writer.write_json(&original, &original_path).unwrap();

        // Convert COCO JSON → Arrow
        let arrow_path = temp_dir.path().join("converted.arrow");
        let coco_options = CocoToArrowOptions {
            include_masks: false,
            group: Some("train".to_string()),
            ..Default::default()
        };

        let count = coco_to_arrow(&original_path, &arrow_path, &coco_options, None)
            .await
            .unwrap();
        assert_eq!(count, 3);

        // Convert Arrow → COCO JSON
        let restored_path = temp_dir.path().join("restored.json");
        let arrow_options = ArrowToCocoOptions {
            include_masks: false,
            ..Default::default()
        };

        arrow_to_coco(&arrow_path, &restored_path, &arrow_options, None)
            .await
            .unwrap();

        // Read the round-tripped JSON
        let reader = CocoReader::new();
        let restored = reader.read_json(&restored_path).unwrap();

        // Verify counts
        assert_eq!(restored.categories.len(), 3);
        assert_eq!(restored.annotations.len(), 3);

        // Verify category IDs are preserved (not renumbered to 1, 2, 3)
        let restored_cat_ids: std::collections::HashSet<u32> =
            restored.categories.iter().map(|c| c.id).collect();
        assert!(
            restored_cat_ids.contains(&1),
            "Category ID 1 should be preserved"
        );
        assert!(
            restored_cat_ids.contains(&3),
            "Category ID 3 should be preserved (not renumbered to 2)"
        );
        assert!(
            restored_cat_ids.contains(&90),
            "Category ID 90 should be preserved (not renumbered to 3)"
        );

        // Verify category names map to correct IDs
        for cat in &restored.categories {
            match cat.name.as_str() {
                "person" => assert_eq!(cat.id, 1, "person should have category_id 1"),
                "car" => assert_eq!(cat.id, 3, "car should have category_id 3"),
                "toothbrush" => assert_eq!(cat.id, 90, "toothbrush should have category_id 90"),
                other => panic!("Unexpected category name: {other}"),
            }
        }

        // Verify annotations reference the correct (preserved) category IDs
        let ann_cat_ids: std::collections::HashSet<u32> =
            restored.annotations.iter().map(|a| a.category_id).collect();
        assert!(
            ann_cat_ids.contains(&1),
            "Annotation should reference category_id 1"
        );
        assert!(
            ann_cat_ids.contains(&3),
            "Annotation should reference category_id 3"
        );
        assert!(
            ann_cat_ids.contains(&90),
            "Annotation should reference category_id 90"
        );
    }

    /// Test CocoIndex frequency lookup for LVIS category frequency groups.
    #[test]
    fn test_coco_index_frequency_lookup() {
        let dataset = CocoDataset {
            categories: vec![
                CocoCategory {
                    id: 1,
                    name: "person".to_string(),
                    frequency: Some("f".to_string()),
                    ..Default::default()
                },
                CocoCategory {
                    id: 3,
                    name: "car".to_string(),
                    frequency: Some("c".to_string()),
                    ..Default::default()
                },
                CocoCategory {
                    id: 90,
                    name: "toothbrush".to_string(),
                    frequency: Some("r".to_string()),
                    ..Default::default()
                },
            ],
            ..Default::default()
        };

        let index = CocoIndex::from_dataset(&dataset);
        assert_eq!(index.frequency(1), Some("f"));
        assert_eq!(index.frequency(3), Some("c"));
        assert_eq!(index.frequency(90), Some("r"));
        assert_eq!(index.frequency(999), None); // unknown category
    }

    /// Test that old Arrow files (2025.10 schema without LVIS columns) can be
    /// read without error. New columns are simply absent — not an error.
    #[cfg(feature = "polars")]
    #[test]
    fn test_read_old_arrow_without_lvis_columns() {
        use polars::prelude::*;
        use tempfile::TempDir;

        // Build minimal 2025.10-style DataFrame (no LVIS columns)
        let df = DataFrame::new_infer_height(vec![
            Series::new("name".into(), vec!["test"]).into(),
            Series::new("label".into(), vec![Some("person")]).into(),
            Series::new("label_index".into(), vec![Some(1u64)]).into(),
        ])
        .unwrap();

        let dir = TempDir::new().unwrap();
        let path = dir.path().join("old_format.arrow");

        // Write without metadata (simulating old file)
        let mut file = std::fs::File::create(&path).unwrap();
        IpcWriter::new(&mut file).finish(&mut df.clone()).unwrap();

        // Read back — should succeed, new columns are just absent
        let mut file = std::fs::File::open(&path).unwrap();
        let read_df = IpcReader::new(&mut file).finish().unwrap();

        assert!(read_df.column("name").is_ok());
        assert!(read_df.column("label").is_ok());
        // New LVIS columns should be absent (not error, just not present)
        assert!(read_df.column("iscrowd").is_err());
        assert!(read_df.column("category_frequency").is_err());
        assert!(read_df.column("neg_label_indices").is_err());
        assert!(read_df.column("not_exhaustive_label_indices").is_err());
    }

    /// Test full round-trip of LVIS fields: COCO JSON → Arrow → COCO JSON
    #[cfg(feature = "polars")]
    #[tokio::test]
    async fn test_coco_arrow_roundtrip_lvis_fields() {
        let dataset = CocoDataset {
            images: vec![CocoImage {
                id: 1,
                width: 640,
                height: 480,
                file_name: "test.jpg".to_string(),
                neg_category_ids: Some(vec![3]),
                not_exhaustive_category_ids: Some(vec![90]),
                ..Default::default()
            }],
            categories: vec![
                CocoCategory {
                    id: 1,
                    name: "person".to_string(),
                    frequency: Some("f".to_string()),
                    synset: Some("person.n.01".to_string()),
                    def: Some("a human being".to_string()),
                    ..Default::default()
                },
                CocoCategory {
                    id: 3,
                    name: "car".to_string(),
                    frequency: Some("c".to_string()),
                    ..Default::default()
                },
                CocoCategory {
                    id: 90,
                    name: "toothbrush".to_string(),
                    frequency: Some("r".to_string()),
                    ..Default::default()
                },
            ],
            annotations: vec![CocoAnnotation {
                id: 1,
                image_id: 1,
                category_id: 1,
                bbox: [10.0, 20.0, 100.0, 200.0],
                area: 20000.0,
                iscrowd: 0,
                segmentation: None,
            }],
            ..Default::default()
        };

        let dir = TempDir::new().unwrap();
        let json_in = dir.path().join("input.json");
        let arrow_path = dir.path().join("output.arrow");
        let json_out = dir.path().join("roundtrip.json");

        // Write source COCO with LVIS fields
        let writer = CocoWriter::new();
        writer.write_json(&dataset, &json_in).unwrap();

        // COCO -> Arrow
        let options = CocoToArrowOptions::default();
        coco_to_arrow(&json_in, &arrow_path, &options, None)
            .await
            .unwrap();

        // Arrow -> COCO
        let export_options = ArrowToCocoOptions::default();
        arrow_to_coco(&arrow_path, &json_out, &export_options, None)
            .await
            .unwrap();

        // Read round-tripped COCO
        let reader = CocoReader::new();
        let result = reader.read_json(&json_out).unwrap();

        // Verify LVIS image fields
        let img = &result.images[0];
        assert_eq!(img.neg_category_ids, Some(vec![3]));
        assert_eq!(img.not_exhaustive_category_ids, Some(vec![90]));

        // Verify LVIS category fields
        let person = result
            .categories
            .iter()
            .find(|c| c.name == "person")
            .unwrap();
        assert_eq!(person.frequency, Some("f".to_string()));
        assert_eq!(person.synset, Some("person.n.01".to_string()));
        assert_eq!(person.def, Some("a human being".to_string()));

        let car = result.categories.iter().find(|c| c.name == "car").unwrap();
        assert_eq!(car.frequency, Some("c".to_string()));
    }
}
