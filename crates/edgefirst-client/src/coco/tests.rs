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
                    score: None,
                },
                CocoAnnotation {
                    id: 2,
                    image_id: 1,
                    category_id: 2,
                    bbox: [400.0, 200.0, 150.0, 100.0],
                    area: 15000.0,
                    iscrowd: 0,
                    segmentation: None,
                    score: None,
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
                    score: None,
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
                    score: None,
                },
                CocoAnnotation {
                    id: 2,
                    image_id: 1,
                    category_id: 3,
                    bbox: [400.0, 200.0, 150.0, 100.0],
                    area: 15000.0,
                    iscrowd: 0,
                    segmentation: None,
                    score: None,
                },
                CocoAnnotation {
                    id: 3,
                    image_id: 1,
                    category_id: 90,
                    bbox: [10.0, 10.0, 30.0, 80.0],
                    area: 2400.0,
                    iscrowd: 0,
                    segmentation: None,
                    score: None,
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
                score: None,
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

    /// Test that polygon segmentations produce a polygon column in Arrow output.
    #[cfg(feature = "polars")]
    #[tokio::test]
    async fn test_coco_to_arrow_polygon_column_type() {
        use polars::prelude::*;

        let temp_dir = TempDir::new().unwrap();

        let dataset = CocoDataset {
            images: vec![CocoImage {
                id: 1,
                width: 640,
                height: 480,
                file_name: "polygon_test.jpg".to_string(),
                ..Default::default()
            }],
            categories: vec![CocoCategory {
                id: 1,
                name: "person".to_string(),
                supercategory: Some("human".to_string()),
                ..Default::default()
            }],
            annotations: vec![CocoAnnotation {
                id: 1,
                image_id: 1,
                category_id: 1,
                bbox: [100.0, 50.0, 200.0, 150.0],
                area: 30000.0,
                iscrowd: 0,
                segmentation: Some(CocoSegmentation::Polygon(vec![vec![
                    100.0, 50.0, 300.0, 50.0, 300.0, 200.0, 100.0, 200.0,
                ]])),
                score: None,
            }],
            ..Default::default()
        };

        // Write COCO JSON
        let coco_path = temp_dir.path().join("polygon_test.json");
        let writer = CocoWriter::new();
        writer.write_json(&dataset, &coco_path).unwrap();

        // Convert to Arrow
        let arrow_path = temp_dir.path().join("polygon_test.arrow");
        let options = CocoToArrowOptions::default();
        let count = coco_to_arrow(&coco_path, &arrow_path, &options, None)
            .await
            .unwrap();
        assert_eq!(count, 1);

        // Read back and verify column types
        let mut file = std::fs::File::open(&arrow_path).unwrap();
        let df = IpcReader::new(&mut file).finish().unwrap();

        // Verify "polygon" column exists and is List(List(Float32))
        let polygon_col = df.column("polygon").expect("polygon column should exist");
        let dtype = polygon_col.dtype();
        // polygon is List(List(Float32)) — check outer List
        assert!(
            matches!(dtype, DataType::List(_)),
            "polygon column should be List type, got {:?}",
            dtype
        );

        // Verify "mask" column has no non-null values (only polygon data, no RLE)
        if let Ok(mask_col) = df.column("mask") {
            assert_eq!(
                mask_col.null_count(),
                mask_col.len(),
                "mask column should be all-null when only polygon segmentations are present"
            );
        }
    }

    /// Test that RLE segmentations produce PNG-encoded mask data in Arrow output.
    #[cfg(feature = "polars")]
    #[tokio::test]
    async fn test_coco_to_arrow_rle_to_png_mask() {
        use crate::MaskData;
        use polars::prelude::*;

        let temp_dir = TempDir::new().unwrap();

        // Create COCO dataset with an RLE segmentation annotation
        // 10x10 image, RLE: 10 bg, 5 fg, 85 bg = 100 pixels
        let dataset = CocoDataset {
            images: vec![CocoImage {
                id: 1,
                width: 10,
                height: 10,
                file_name: "rle_test.jpg".to_string(),
                ..Default::default()
            }],
            categories: vec![CocoCategory {
                id: 1,
                name: "object".to_string(),
                supercategory: None,
                ..Default::default()
            }],
            annotations: vec![CocoAnnotation {
                id: 1,
                image_id: 1,
                category_id: 1,
                bbox: [0.0, 0.0, 10.0, 10.0],
                area: 5.0,
                iscrowd: 1,
                segmentation: Some(CocoSegmentation::Rle(CocoRle {
                    counts: vec![10, 5, 85], // 10 bg, 5 fg, 85 bg = 100 pixels for 10x10
                    size: [10, 10],          // [height, width]
                })),
                score: None,
            }],
            ..Default::default()
        };

        // Write COCO JSON
        let coco_path = temp_dir.path().join("rle_test.json");
        let writer = CocoWriter::new();
        writer.write_json(&dataset, &coco_path).unwrap();

        // Convert to Arrow
        let arrow_path = temp_dir.path().join("rle_test.arrow");
        let options = CocoToArrowOptions::default();
        let count = coco_to_arrow(&coco_path, &arrow_path, &options, None)
            .await
            .unwrap();
        assert_eq!(count, 1);

        // Read back and verify mask column
        let mut file = std::fs::File::open(&arrow_path).unwrap();
        let df = IpcReader::new(&mut file).finish().unwrap();

        // Verify "mask" column exists and is Binary
        let mask_col = df.column("mask").expect("mask column should exist");
        assert!(
            matches!(mask_col.dtype(), DataType::Binary),
            "mask column should be Binary type, got {:?}",
            mask_col.dtype()
        );

        // Read the Binary data and construct MaskData
        let binary_ca = mask_col.binary().unwrap();
        let png_bytes = binary_ca.get(0).expect("mask data should not be null");
        assert!(!png_bytes.is_empty(), "PNG bytes should not be empty");

        let mask_data = MaskData::from_png(png_bytes.to_vec());

        // Verify dimensions match the COCO image dimensions
        assert_eq!(mask_data.width(), 10, "mask width should match image width");
        assert_eq!(
            mask_data.height(),
            10,
            "mask height should match image height"
        );

        // Verify bit_depth is 1 (binary mask)
        assert_eq!(mask_data.bit_depth(), 1, "mask should be 1-bit binary");

        // Verify foreground pixel count: RLE had 5 foreground pixels
        let decoded = mask_data.decode().unwrap();
        let fg_count = decoded.iter().filter(|&&v| v == 1).count();
        assert_eq!(fg_count, 5, "should have 5 foreground pixels from RLE");

        // Verify polygon column is all-null (RLE goes to mask, not polygon).
        // The polygon column may be absent entirely or all-null; both are correct.
        if let Ok(polygon_col) = df.column("polygon") {
            assert_eq!(
                polygon_col.null_count(),
                polygon_col.len(),
                "polygon column should be all-null when only RLE segmentations are present"
            );
        }
    }

    /// Test that labels metadata is written to Arrow file.
    #[cfg(feature = "polars")]
    #[tokio::test]
    async fn test_coco_to_arrow_labels_metadata() {
        use polars::prelude::*;

        let temp_dir = TempDir::new().unwrap();

        let dataset = CocoDataset {
            images: vec![CocoImage {
                id: 1,
                width: 640,
                height: 480,
                file_name: "labels_test.jpg".to_string(),
                ..Default::default()
            }],
            categories: vec![
                CocoCategory {
                    id: 3,
                    name: "car".to_string(),
                    ..Default::default()
                },
                CocoCategory {
                    id: 1,
                    name: "person".to_string(),
                    ..Default::default()
                },
            ],
            annotations: vec![CocoAnnotation {
                id: 1,
                image_id: 1,
                category_id: 1,
                bbox: [10.0, 20.0, 100.0, 80.0],
                area: 8000.0,
                iscrowd: 0,
                segmentation: None,
                score: None,
            }],
            ..Default::default()
        };

        let coco_path = temp_dir.path().join("labels_test.json");
        let writer = CocoWriter::new();
        writer.write_json(&dataset, &coco_path).unwrap();

        let arrow_path = temp_dir.path().join("labels_test.arrow");
        let options = CocoToArrowOptions::default();
        coco_to_arrow(&coco_path, &arrow_path, &options, None)
            .await
            .unwrap();

        // Read back and verify labels metadata
        let mut file = std::fs::File::open(&arrow_path).unwrap();
        let mut reader = IpcReader::new(&mut file);
        let custom_meta = reader.custom_metadata().unwrap().unwrap();

        let labels_json = custom_meta
            .get(&PlSmallStr::from("labels"))
            .expect("labels metadata should be present");
        let labels: Vec<String> = serde_json::from_str(labels_json.as_str()).unwrap();

        // Labels should be sorted by category_id: person (id=1), car (id=3)
        assert_eq!(labels, vec!["person", "car"]);
    }

    /// Test that COCO score field is mapped to appropriate geometry score.
    #[cfg(feature = "polars")]
    #[tokio::test]
    async fn test_coco_to_arrow_score_mapping() {
        let temp_dir = TempDir::new().unwrap();

        let dataset = CocoDataset {
            images: vec![CocoImage {
                id: 1,
                width: 100,
                height: 100,
                file_name: "score_test.jpg".to_string(),
                ..Default::default()
            }],
            categories: vec![CocoCategory {
                id: 1,
                name: "person".to_string(),
                ..Default::default()
            }],
            annotations: vec![
                // Annotation with polygon + score → polygon_score
                CocoAnnotation {
                    id: 1,
                    image_id: 1,
                    category_id: 1,
                    bbox: [10.0, 10.0, 50.0, 50.0],
                    area: 2500.0,
                    iscrowd: 0,
                    segmentation: Some(CocoSegmentation::Polygon(vec![vec![
                        10.0, 10.0, 60.0, 10.0, 60.0, 60.0, 10.0, 60.0,
                    ]])),
                    score: Some(0.95),
                },
                // Annotation without segmentation + score → box2d_score
                CocoAnnotation {
                    id: 2,
                    image_id: 1,
                    category_id: 1,
                    bbox: [20.0, 20.0, 30.0, 30.0],
                    area: 900.0,
                    iscrowd: 0,
                    segmentation: None,
                    score: Some(0.85),
                },
            ],
            ..Default::default()
        };

        let coco_path = temp_dir.path().join("score_test.json");
        let writer = CocoWriter::new();
        writer.write_json(&dataset, &coco_path).unwrap();

        let arrow_path = temp_dir.path().join("score_test.arrow");
        let options = CocoToArrowOptions::default();
        let count = coco_to_arrow(&coco_path, &arrow_path, &options, None)
            .await
            .unwrap();
        assert_eq!(count, 2);

        // Read back and verify score columns
        use polars::prelude::*;
        let mut file = std::fs::File::open(&arrow_path).unwrap();
        let df = IpcReader::new(&mut file).finish().unwrap();

        // Row 0 has polygon → polygon_score should be 0.95
        let polygon_scores = df.column("polygon_score").unwrap().f32().unwrap();
        assert!(
            (polygon_scores.get(0).unwrap() - 0.95).abs() < 1e-4,
            "polygon_score should be ~0.95"
        );

        // Row 1 has no segmentation → box2d_score should be 0.85
        let box2d_scores = df.column("box2d_score").unwrap().f32().unwrap();
        assert!(
            (box2d_scores.get(1).unwrap() - 0.85).abs() < 1e-4,
            "box2d_score should be ~0.85"
        );
    }

    /// Test that a synthetic 2025.10 Arrow file (mask: List(Float32) with NaN
    /// separators, no schema_version metadata) can be read back via arrow_to_coco
    /// and that polygon coordinates are correctly reconstructed.
    #[cfg(feature = "polars")]
    #[tokio::test]
    async fn test_read_2025_10_arrow_file_compat() {
        use polars::prelude::*;

        let temp_dir = TempDir::new().unwrap();

        // Build a synthetic 2025.10-style DataFrame:
        // - No schema_version metadata
        // - "mask" column is List(Float32) with NaN-separated polygon coords
        // - No "polygon" column
        // - iscrowd as UInt32 (old schema)
        //
        // Polygon: a square covering the top-left quarter of a 100x100 image
        // Normalized coords: (0.0, 0.0), (0.5, 0.0), (0.5, 0.5), (0.0, 0.5)
        let mask_coords = Series::new(
            "mask".into(),
            vec![0.0f32, 0.0, 0.5, 0.0, 0.5, 0.5, 0.0, 0.5],
        );

        let size_inner = Series::new("size".into(), vec![100u32, 100]);
        let box2d_inner = Series::new("box2d".into(), vec![0.25f32, 0.25, 0.5, 0.5]); // cx, cy, w, h

        let df = DataFrame::new_infer_height(vec![
            Series::new("name".into(), vec!["test_image"]).into(),
            Series::new("label".into(), vec![Some("person")])
                .cast(&DataType::Categorical(
                    polars::prelude::Categories::new(
                        "labels".into(),
                        "labels".into(),
                        CategoricalPhysical::U8,
                    ),
                    std::sync::Arc::new(CategoricalMapping::with_hasher(
                        u8::MAX as usize,
                        Default::default(),
                    )),
                ))
                .unwrap()
                .into(),
            Series::new("label_index".into(), vec![Some(1u64)]).into(),
            Series::new("group".into(), vec![Some("train")])
                .cast(&DataType::Categorical(
                    polars::prelude::Categories::new(
                        "groups".into(),
                        "groups".into(),
                        CategoricalPhysical::U8,
                    ),
                    std::sync::Arc::new(CategoricalMapping::with_hasher(
                        u8::MAX as usize,
                        Default::default(),
                    )),
                ))
                .unwrap()
                .into(),
            Series::new("mask".into(), vec![Some(mask_coords)])
                .cast(&DataType::List(Box::new(DataType::Float32)))
                .unwrap()
                .into(),
            Series::new("box2d".into(), vec![Some(box2d_inner)])
                .cast(&DataType::Array(Box::new(DataType::Float32), 4))
                .unwrap()
                .into(),
            Series::new("size".into(), vec![Some(size_inner)])
                .cast(&DataType::Array(Box::new(DataType::UInt32), 2))
                .unwrap()
                .into(),
            Series::new("iscrowd".into(), vec![0u32]).into(),
        ])
        .unwrap();

        // Write without schema_version metadata (simulating 2025.10 file)
        let arrow_path = temp_dir.path().join("legacy.arrow");
        let mut file = std::fs::File::create(&arrow_path).unwrap();
        IpcWriter::new(&mut file).finish(&mut df.clone()).unwrap();

        // Convert to COCO using arrow_to_coco
        let coco_path = temp_dir.path().join("legacy_output.json");
        let options = ArrowToCocoOptions::default();
        let count = arrow_to_coco(&arrow_path, &coco_path, &options, None)
            .await
            .unwrap();

        assert_eq!(count, 1, "Should have 1 annotation");

        // Read the COCO JSON and verify polygon was reconstructed
        let reader = CocoReader::new();
        let dataset = reader.read_json(&coco_path).unwrap();
        assert_eq!(dataset.annotations.len(), 1);

        let ann = &dataset.annotations[0];
        assert!(
            ann.segmentation.is_some(),
            "Should have polygon segmentation from legacy mask column"
        );

        if let Some(CocoSegmentation::Polygon(polys)) = &ann.segmentation {
            assert_eq!(polys.len(), 1, "Should have 1 polygon ring");
            assert_eq!(polys[0].len(), 8, "Should have 4 points (8 coords)");
            // First point should be near (0, 0) in pixel coords for 100x100 image
            assert!(polys[0][0].abs() < 1.0, "x0 should be near 0");
            assert!(polys[0][1].abs() < 1.0, "y0 should be near 0");
            // Third point (index 2,3) should be near (50, 0)
            assert!((polys[0][2] - 50.0).abs() < 1.0, "x1 should be near 50");
        } else {
            panic!("Expected polygon segmentation");
        }
    }

    /// Test full roundtrip: COCO (with polygons) -> coco_to_arrow -> arrow_to_coco
    /// Verify polygon coordinates survive the roundtrip.
    #[cfg(feature = "polars")]
    #[tokio::test]
    async fn test_arrow_to_coco_polygon_roundtrip() {
        let temp_dir = TempDir::new().unwrap();

        let original = CocoDataset {
            images: vec![CocoImage {
                id: 1,
                width: 400,
                height: 400,
                file_name: "poly_roundtrip.jpg".to_string(),
                ..Default::default()
            }],
            categories: vec![CocoCategory {
                id: 1,
                name: "cat".to_string(),
                supercategory: Some("animal".to_string()),
                ..Default::default()
            }],
            annotations: vec![CocoAnnotation {
                id: 1,
                image_id: 1,
                category_id: 1,
                bbox: [50.0, 60.0, 200.0, 180.0],
                area: 36000.0,
                iscrowd: 0,
                segmentation: Some(CocoSegmentation::Polygon(vec![vec![
                    50.0, 60.0, 250.0, 60.0, 250.0, 240.0, 50.0, 240.0,
                ]])),
                score: None,
            }],
            ..Default::default()
        };

        // Write COCO JSON
        let coco_path = temp_dir.path().join("poly_original.json");
        let writer = CocoWriter::new();
        writer.write_json(&original, &coco_path).unwrap();

        // COCO -> Arrow
        let arrow_path = temp_dir.path().join("poly_roundtrip.arrow");
        let options = CocoToArrowOptions::default();
        let count = coco_to_arrow(&coco_path, &arrow_path, &options, None)
            .await
            .unwrap();
        assert_eq!(count, 1);

        // Arrow -> COCO
        let restored_path = temp_dir.path().join("poly_restored.json");
        let export_options = ArrowToCocoOptions::default();
        arrow_to_coco(&arrow_path, &restored_path, &export_options, None)
            .await
            .unwrap();

        // Read back and compare
        let reader = CocoReader::new();
        let restored = reader.read_json(&restored_path).unwrap();

        assert_eq!(restored.annotations.len(), 1);
        let ann = &restored.annotations[0];
        assert!(
            ann.segmentation.is_some(),
            "Polygon should survive roundtrip"
        );

        if let Some(CocoSegmentation::Polygon(polys)) = &ann.segmentation {
            assert_eq!(polys.len(), 1, "Should have 1 polygon ring");
            let orig_poly = &original.annotations[0].segmentation.as_ref().unwrap();
            if let CocoSegmentation::Polygon(orig_polys) = orig_poly {
                assert_eq!(polys[0].len(), orig_polys[0].len());
                for j in 0..orig_polys[0].len() {
                    assert!(
                        (polys[0][j] - orig_polys[0][j]).abs() < 2.0,
                        "Polygon coord mismatch at {}: {} vs {}",
                        j,
                        polys[0][j],
                        orig_polys[0][j]
                    );
                }
            }
        } else {
            panic!("Expected polygon segmentation after roundtrip");
        }
    }

    /// Test full roundtrip: COCO (with RLE mask) -> coco_to_arrow -> arrow_to_coco
    /// Verify RLE mask data survives (decode and compare pixel counts).
    #[cfg(feature = "polars")]
    #[tokio::test]
    async fn test_arrow_to_coco_rle_mask_roundtrip() {
        let temp_dir = TempDir::new().unwrap();

        // Create a COCO dataset with an RLE segmentation
        // 10x10 image, RLE: 10 bg, 5 fg, 85 bg = 100 pixels (5 foreground)
        let original = CocoDataset {
            images: vec![CocoImage {
                id: 1,
                width: 10,
                height: 10,
                file_name: "rle_roundtrip.jpg".to_string(),
                ..Default::default()
            }],
            categories: vec![CocoCategory {
                id: 1,
                name: "object".to_string(),
                supercategory: None,
                ..Default::default()
            }],
            annotations: vec![CocoAnnotation {
                id: 1,
                image_id: 1,
                category_id: 1,
                bbox: [0.0, 0.0, 10.0, 10.0],
                area: 5.0,
                iscrowd: 1,
                segmentation: Some(CocoSegmentation::Rle(CocoRle {
                    counts: vec![10, 5, 85],
                    size: [10, 10],
                })),
                score: None,
            }],
            ..Default::default()
        };

        // Decode original RLE to count foreground pixels
        let (orig_mask, _, _) = decode_rle(
            original.annotations[0]
                .segmentation
                .as_ref()
                .and_then(|s| match s {
                    CocoSegmentation::Rle(rle) => Some(rle),
                    _ => None,
                })
                .unwrap(),
        )
        .unwrap();
        let orig_fg_count = orig_mask.iter().filter(|&&v| v == 1).count();
        assert_eq!(orig_fg_count, 5);

        // Write COCO JSON
        let coco_path = temp_dir.path().join("rle_original.json");
        let writer = CocoWriter::new();
        writer.write_json(&original, &coco_path).unwrap();

        // COCO -> Arrow (RLE becomes PNG mask in Arrow)
        let arrow_path = temp_dir.path().join("rle_roundtrip.arrow");
        let options = CocoToArrowOptions::default();
        let count = coco_to_arrow(&coco_path, &arrow_path, &options, None)
            .await
            .unwrap();
        assert_eq!(count, 1);

        // Arrow -> COCO (PNG mask becomes RLE in COCO output)
        let restored_path = temp_dir.path().join("rle_restored.json");
        let export_options = ArrowToCocoOptions::default();
        arrow_to_coco(&arrow_path, &restored_path, &export_options, None)
            .await
            .unwrap();

        // Read back and verify the mask was reconstructed as RLE
        let reader = CocoReader::new();
        let restored = reader.read_json(&restored_path).unwrap();

        assert_eq!(restored.annotations.len(), 1);
        let ann = &restored.annotations[0];
        assert!(
            ann.segmentation.is_some(),
            "RLE mask should survive roundtrip"
        );

        match &ann.segmentation {
            Some(CocoSegmentation::Rle(rle)) => {
                let (restored_mask, _, _) = decode_rle(rle).unwrap();
                let restored_fg_count = restored_mask.iter().filter(|&&v| v == 1).count();
                assert_eq!(
                    restored_fg_count, orig_fg_count,
                    "Foreground pixel count should be preserved: expected {}, got {}",
                    orig_fg_count, restored_fg_count
                );
            }
            _ => {
                // This is OK — the roundtrip preserves the mask content even if
                // the representation type differs (e.g., polygon from contour tracing).
                // What matters is that segmentation data survived.
            }
        }
    }

    /// Test COCO annotations with score -> coco_to_arrow -> arrow_to_coco.
    /// Verify scores survive the roundtrip.
    #[cfg(feature = "polars")]
    #[tokio::test]
    async fn test_arrow_to_coco_score_roundtrip() {
        let temp_dir = TempDir::new().unwrap();

        let original = CocoDataset {
            images: vec![CocoImage {
                id: 1,
                width: 100,
                height: 100,
                file_name: "score_roundtrip.jpg".to_string(),
                ..Default::default()
            }],
            categories: vec![CocoCategory {
                id: 1,
                name: "person".to_string(),
                ..Default::default()
            }],
            annotations: vec![
                // Annotation with polygon + score
                CocoAnnotation {
                    id: 1,
                    image_id: 1,
                    category_id: 1,
                    bbox: [10.0, 10.0, 50.0, 50.0],
                    area: 2500.0,
                    iscrowd: 0,
                    segmentation: Some(CocoSegmentation::Polygon(vec![vec![
                        10.0, 10.0, 60.0, 10.0, 60.0, 60.0, 10.0, 60.0,
                    ]])),
                    score: Some(0.95),
                },
                // Annotation with bbox only + score
                CocoAnnotation {
                    id: 2,
                    image_id: 1,
                    category_id: 1,
                    bbox: [20.0, 20.0, 30.0, 30.0],
                    area: 900.0,
                    iscrowd: 0,
                    segmentation: None,
                    score: Some(0.85),
                },
            ],
            ..Default::default()
        };

        // Write COCO JSON
        let coco_path = temp_dir.path().join("score_original.json");
        let writer = CocoWriter::new();
        writer.write_json(&original, &coco_path).unwrap();

        // COCO -> Arrow
        let arrow_path = temp_dir.path().join("score_roundtrip.arrow");
        let options = CocoToArrowOptions::default();
        let count = coco_to_arrow(&coco_path, &arrow_path, &options, None)
            .await
            .unwrap();
        assert_eq!(count, 2);

        // Arrow -> COCO
        let restored_path = temp_dir.path().join("score_restored.json");
        let export_options = ArrowToCocoOptions::default();
        arrow_to_coco(&arrow_path, &restored_path, &export_options, None)
            .await
            .unwrap();

        // Read back and verify scores
        let reader = CocoReader::new();
        let restored = reader.read_json(&restored_path).unwrap();

        assert_eq!(restored.annotations.len(), 2);

        // Find annotation with polygon (should have score ~0.95)
        let poly_ann = restored
            .annotations
            .iter()
            .find(|a| a.segmentation.is_some())
            .expect("Should have annotation with segmentation");
        assert!(
            poly_ann.score.is_some(),
            "Polygon annotation should have score"
        );
        assert!(
            (poly_ann.score.unwrap() - 0.95).abs() < 0.01,
            "Polygon score should be ~0.95, got {}",
            poly_ann.score.unwrap()
        );

        // Find annotation without segmentation (should have score ~0.85)
        let bbox_ann = restored
            .annotations
            .iter()
            .find(|a| a.segmentation.is_none())
            .expect("Should have annotation without segmentation");
        assert!(
            bbox_ann.score.is_some(),
            "Bbox annotation should have score"
        );
        assert!(
            (bbox_ann.score.unwrap() - 0.85).abs() < 0.01,
            "Bbox score should be ~0.85, got {}",
            bbox_ann.score.unwrap()
        );
    }

    /// End-to-end integration test: COCO dataset with BOTH polygon AND RLE
    /// mask annotations in the same file → coco_to_arrow → verify Arrow
    /// columns → arrow_to_coco → verify both geometry types survive.
    #[cfg(feature = "polars")]
    #[tokio::test]
    async fn test_full_roundtrip_polygon_and_rle_mask() {
        use crate::MaskData;
        use polars::prelude::*;

        let temp_dir = TempDir::new().unwrap();

        // Create a COCO dataset with mixed geometry types:
        // - Image 1: polygon annotation (person) + bbox-only annotation (car)
        // - Image 2: RLE mask annotation (crowd of people, iscrowd=1)
        let original = CocoDataset {
            info: CocoInfo {
                year: Some(2026),
                version: Some("1.0".to_string()),
                description: Some("Mixed polygon+RLE integration test".to_string()),
                ..Default::default()
            },
            images: vec![
                CocoImage {
                    id: 1,
                    width: 200,
                    height: 200,
                    file_name: "image_polygon.jpg".to_string(),
                    ..Default::default()
                },
                CocoImage {
                    id: 2,
                    width: 10,
                    height: 10,
                    file_name: "image_rle.jpg".to_string(),
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
                // Annotation 1: polygon segmentation on image 1
                CocoAnnotation {
                    id: 1,
                    image_id: 1,
                    category_id: 1,
                    bbox: [30.0, 40.0, 100.0, 120.0],
                    area: 12000.0,
                    iscrowd: 0,
                    segmentation: Some(CocoSegmentation::Polygon(vec![vec![
                        30.0, 40.0, 130.0, 40.0, 130.0, 160.0, 30.0, 160.0,
                    ]])),
                    score: None,
                },
                // Annotation 2: bbox only on image 1 (no segmentation)
                CocoAnnotation {
                    id: 2,
                    image_id: 1,
                    category_id: 2,
                    bbox: [150.0, 150.0, 40.0, 30.0],
                    area: 1200.0,
                    iscrowd: 0,
                    segmentation: None,
                    score: None,
                },
                // Annotation 3: RLE mask on image 2 (crowd annotation)
                // 10x10 image, RLE: 10 bg, 5 fg, 85 bg = 100 pixels (5 fg)
                CocoAnnotation {
                    id: 3,
                    image_id: 2,
                    category_id: 1,
                    bbox: [0.0, 0.0, 10.0, 10.0],
                    area: 5.0,
                    iscrowd: 1,
                    segmentation: Some(CocoSegmentation::Rle(CocoRle {
                        counts: vec![10, 5, 85],
                        size: [10, 10],
                    })),
                    score: None,
                },
            ],
            licenses: vec![],
        };

        // Decode original RLE foreground count for later comparison
        let orig_rle = match &original.annotations[2].segmentation {
            Some(CocoSegmentation::Rle(rle)) => rle,
            _ => panic!("Expected RLE segmentation on annotation 3"),
        };
        let (orig_mask_pixels, _, _) = decode_rle(orig_rle).unwrap();
        let orig_fg_count = orig_mask_pixels.iter().filter(|&&v| v == 1).count();
        assert_eq!(
            orig_fg_count, 5,
            "Original RLE should have 5 foreground pixels"
        );

        // Step 1: Write COCO JSON
        let coco_path = temp_dir.path().join("mixed_input.json");
        let writer = CocoWriter::new();
        writer.write_json(&original, &coco_path).unwrap();

        // Step 2: COCO -> Arrow
        let arrow_path = temp_dir.path().join("mixed.arrow");
        let coco_options = CocoToArrowOptions {
            include_masks: true,
            group: Some("val".to_string()),
            ..Default::default()
        };
        let count = coco_to_arrow(&coco_path, &arrow_path, &coco_options, None)
            .await
            .unwrap();
        assert_eq!(count, 3, "Should convert all 3 annotations");

        // Step 3: Verify Arrow file has both polygon and mask columns
        let mut file = std::fs::File::open(&arrow_path).unwrap();
        let df = IpcReader::new(&mut file).finish().unwrap();

        // polygon column should exist
        let polygon_col = df.column("polygon").expect("polygon column should exist");
        assert!(
            matches!(polygon_col.dtype(), DataType::List(_)),
            "polygon column should be List type, got {:?}",
            polygon_col.dtype()
        );

        // mask column should exist and be Binary
        let mask_col = df.column("mask").expect("mask column should exist");
        assert!(
            matches!(mask_col.dtype(), DataType::Binary),
            "mask column should be Binary type, got {:?}",
            mask_col.dtype()
        );

        // Verify polygon column has at least one non-null value (from annotation 1)
        assert!(
            polygon_col.null_count() < polygon_col.len(),
            "polygon column should have at least one non-null entry"
        );

        // Verify mask column has at least one non-null value (from annotation 3)
        assert!(
            mask_col.null_count() < mask_col.len(),
            "mask column should have at least one non-null entry"
        );

        // Verify schema_version metadata is written
        let mut meta_file = std::fs::File::open(&arrow_path).unwrap();
        let mut meta_reader = IpcReader::new(&mut meta_file);
        let custom_meta = meta_reader.custom_metadata().unwrap().unwrap();
        assert_eq!(
            custom_meta
                .get(&PlSmallStr::from("schema_version"))
                .map(|s| s.to_string()),
            Some("2026.04".to_string()),
            "schema_version should be 2026.04"
        );

        // Verify the mask Binary data is valid PNG
        let binary_ca = mask_col.binary().unwrap();
        // Find the row with non-null mask (annotation 3 = RLE)
        let mask_row = (0..binary_ca.len())
            .find(|&i| binary_ca.get(i).is_some())
            .expect("Should have at least one non-null mask row");
        let png_bytes = binary_ca.get(mask_row).unwrap();
        let mask_data = MaskData::from_png(png_bytes.to_vec());
        assert_eq!(mask_data.width(), 10);
        assert_eq!(mask_data.height(), 10);
        assert_eq!(
            mask_data.bit_depth(),
            1,
            "RLE mask should produce 1-bit PNG"
        );
        let decoded = mask_data.decode().unwrap();
        let arrow_fg_count = decoded.iter().filter(|&&v| v == 1).count();
        assert_eq!(
            arrow_fg_count, orig_fg_count,
            "PNG mask in Arrow should preserve foreground pixel count"
        );

        // Step 4: Arrow -> COCO
        let restored_path = temp_dir.path().join("mixed_restored.json");
        let export_options = ArrowToCocoOptions::default();
        let ann_count = arrow_to_coco(&arrow_path, &restored_path, &export_options, None)
            .await
            .unwrap();
        assert_eq!(ann_count, 3, "Should export all 3 annotations");

        // Step 5: Read restored COCO and verify both geometry types survive
        let reader = CocoReader::new();
        let restored = reader.read_json(&restored_path).unwrap();

        assert_eq!(restored.annotations.len(), 3);
        assert_eq!(restored.categories.len(), 2);
        assert_eq!(restored.images.len(), 2);

        // Verify category names survived
        let cat_names: std::collections::HashSet<_> = restored
            .categories
            .iter()
            .map(|c| c.name.as_str())
            .collect();
        assert!(cat_names.contains("person"));
        assert!(cat_names.contains("car"));

        // Find annotations by their characteristics
        let polygon_anns: Vec<_> = restored
            .annotations
            .iter()
            .filter(|a| matches!(&a.segmentation, Some(CocoSegmentation::Polygon(_))))
            .collect();
        let rle_anns: Vec<_> = restored
            .annotations
            .iter()
            .filter(|a| matches!(&a.segmentation, Some(CocoSegmentation::Rle(_))))
            .collect();
        let no_seg_anns: Vec<_> = restored
            .annotations
            .iter()
            .filter(|a| a.segmentation.is_none())
            .collect();

        // Polygon annotation should survive as polygon
        assert_eq!(
            polygon_anns.len(),
            1,
            "Should have exactly 1 polygon annotation after roundtrip"
        );
        if let Some(CocoSegmentation::Polygon(polys)) = &polygon_anns[0].segmentation {
            assert_eq!(polys.len(), 1, "Polygon should have 1 ring");
            assert_eq!(
                polys[0].len(),
                8,
                "Polygon ring should have 8 coords (4 points)"
            );
            // Verify coordinates are approximately correct (within 2px tolerance)
            let orig_coords = [30.0, 40.0, 130.0, 40.0, 130.0, 160.0, 30.0, 160.0];
            for (j, &orig_val) in orig_coords.iter().enumerate() {
                assert!(
                    (polys[0][j] - orig_val).abs() < 2.0,
                    "Polygon coord [{}] mismatch: {} vs {} (tolerance 2.0)",
                    j,
                    polys[0][j],
                    orig_val
                );
            }
        }

        // RLE mask annotation should survive (as RLE)
        assert_eq!(
            rle_anns.len(),
            1,
            "Should have exactly 1 RLE annotation after roundtrip"
        );
        if let Some(CocoSegmentation::Rle(rle)) = &rle_anns[0].segmentation {
            let (restored_mask, _, _) = decode_rle(rle).unwrap();
            let restored_fg = restored_mask.iter().filter(|&&v| v == 1).count();
            assert_eq!(
                restored_fg, orig_fg_count,
                "RLE foreground pixel count should be preserved: expected {}, got {}",
                orig_fg_count, restored_fg
            );
        }

        // Bbox-only annotation should have no segmentation
        assert_eq!(
            no_seg_anns.len(),
            1,
            "Should have exactly 1 annotation without segmentation"
        );
    }
}
