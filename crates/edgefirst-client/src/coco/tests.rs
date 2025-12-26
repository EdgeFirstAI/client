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
                },
                CocoCategory {
                    id: 2,
                    name: "car".to_string(),
                    supercategory: Some("vehicle".to_string()),
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
            ([0.5, 0.5, 1.0, 1.0], 100, 100), // Sub-pixel precision
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
            vec![10.0, 20.0, 100.0, 20.0, 100.0, 100.0, 50.0, 120.0, 10.0, 100.0],
            vec![200.0, 200.0, 250.0, 200.0, 250.0, 250.0, 200.0, 250.0],
        ];

        let image_w = 400;
        let image_h = 400;

        let mask = coco_polygon_to_mask(&original, image_w, image_h);
        let restored = mask_to_coco_polygon(&mask, image_w, image_h);

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
        // Actually, the RLE counts should be: 0 (bg), 2 (fg), 2 (bg), 2 (fg), 10 (bg) = sum 16 ✓

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

    /// Test CocoIndex efficient lookups
    #[test]
    fn test_coco_index_lookups() {
        let dataset = CocoDataset {
            images: vec![
                CocoImage { id: 1, width: 640, height: 480, file_name: "a.jpg".to_string(), ..Default::default() },
                CocoImage { id: 2, width: 800, height: 600, file_name: "b.jpg".to_string(), ..Default::default() },
            ],
            categories: vec![
                CocoCategory { id: 10, name: "zebra".to_string(), supercategory: None },
                CocoCategory { id: 20, name: "apple".to_string(), supercategory: None },
            ],
            annotations: vec![
                CocoAnnotation { id: 100, image_id: 1, category_id: 10, bbox: [0.0; 4], ..Default::default() },
                CocoAnnotation { id: 101, image_id: 1, category_id: 20, bbox: [0.0; 4], ..Default::default() },
                CocoAnnotation { id: 102, image_id: 2, category_id: 10, bbox: [0.0; 4], ..Default::default() },
            ],
            ..Default::default()
        };

        let index = CocoIndex::from_dataset(&dataset);

        // Test image lookup
        assert!(index.images.get(&1).is_some());
        assert!(index.images.get(&999).is_none());

        // Test category lookup
        assert_eq!(index.label_name(10), Some("zebra"));
        assert_eq!(index.label_name(20), Some("apple"));
        assert_eq!(index.label_name(999), None);

        // Test alphabetical label indices (apple=0, zebra=1)
        assert_eq!(index.label_index(20), Some(0)); // apple
        assert_eq!(index.label_index(10), Some(1)); // zebra

        // Test annotations by image
        assert_eq!(index.annotations_for_image(1).len(), 2);
        assert_eq!(index.annotations_for_image(2).len(), 1);
        assert_eq!(index.annotations_for_image(999).len(), 0);
    }
}
