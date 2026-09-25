import 'package:dynamodb_manager/src/models/dynamo_item.dart';
import 'package:dynamodb_manager/src/widgets/dynamo_items_list.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  testWidgets('renders the visible items and reports the selected index', (
    tester,
  ) async {
    final items = List.generate(
      50,
      (index) => DynamoItem.fromDynamoJson(
        '{"pk":"item-$index","payload":"value-$index"}',
      ),
    );
    int? selectedIndex;

    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: DynamoItemsList(
            itemsProvider: () => items,
            selectedItemProvider: () => null,
            onSelect: (index) => selectedIndex = index,
          ),
        ),
      ),
    );

    expect(find.text('pk: item-0'), findsOneWidget);
    await tester.tap(find.text('pk: item-0'));
    expect(selectedIndex, 0);
  });
}
